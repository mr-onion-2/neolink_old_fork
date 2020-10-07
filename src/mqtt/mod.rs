use crossbeam_channel::{unbounded, Receiver, RecvError, Sender};
use log::*;
use rumqttc::{
    Client, ClientError, ConnectReturnCode, Connection, Event, Incoming, MqttOptions, Publish, QoS,
    LastWill,
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use validator::{Validate, ValidationError};
use validator_derive::Validate;

pub struct MQTT {
    client: Mutex<Client>,
    connection: Mutex<Connection>,
    name: String,
    incoming: (Sender<Publish>, Receiver<Publish>),
}

// Currently we don't read replies but we could do command and control with it.
#[allow(dead_code)]
pub struct MqttReply {
    topic: String,
    message: String,
}

#[derive(Debug, Deserialize, Clone, Validate)]
#[validate(schema(function = "validate_mqtt_config", skip_on_field_errors = true))]
pub struct MqttConfig {
    #[serde(alias = "server")]
    broker_addr: String,

    port: u16,

    #[serde(default)]
    credentials: Option<(String, String)>,

    #[serde(default)]
    ca: Option<std::path::PathBuf>,

    #[serde(default)]
    client_auth: Option<(std::path::PathBuf, std::path::PathBuf)>,
}

fn validate_mqtt_config(config: &MqttConfig) -> Result<(), ValidationError> {
    if config.ca.is_some() && config.client_auth.is_some() {
        Err(ValidationError::new(
            "Cannot have both ca and client_auth set",
        ))
    } else {
        Ok(())
    }
}

impl MQTT {
    pub fn new(config: &MqttConfig, name: &str) -> Arc<Self> {
        let incoming = unbounded::<Publish>();
        let mut mqttoptions = MqttOptions::new(
            format!("Neolink-{}", name),
            &config.broker_addr,
            config.port,
        );
        if let Some(ca_path) = &config.ca {
            if let Ok(buf) = std::fs::read(ca_path) {
                mqttoptions.set_ca(buf);
            } else {
                error!("Failed to read CA certificate");
            }
        }

        if let Some((cert_path, key_path)) = &config.client_auth {
            if let (Ok(cert_buf), Ok(key_buf)) = (std::fs::read(cert_path), std::fs::read(key_path))
            {
                mqttoptions.set_client_auth(cert_buf, key_buf);
            } else {
                error!("Failed to set client tls");
            }
        }

        if let Some((username, password)) = &config.credentials {
            mqttoptions.set_credentials(username, password);
        }

        mqttoptions.set_keep_alive(5);
        mqttoptions.set_last_will(LastWill::new(
            format!("neolink/{}/status", name),
            "offline",
            QoS::AtLeastOnce,
            true
        ));
        let (client, connection) = Client::new(mqttoptions, 10);

        let me = Self {
            client: Mutex::new(client),
            connection: Mutex::new(connection),
            name: name.to_string(),
            incoming,
        };

        let arc_me = Arc::new(me);

        // Start the mqtt server
        let mqtt_running = arc_me.clone();
        std::thread::spawn(move || {
            let _ = (*mqtt_running).start();
        });

        // Start polling messages
        let mqtt_reading = arc_me.clone();
        let arc_name = Arc::new(name.to_string());
        std::thread::spawn(move || loop {
            if (*mqtt_reading).get_message().is_err() {
                error!("Failed to get messages from mqtt client {}", (*arc_name));
            }
        });

        arc_me
    }

    fn subscribe(&self) -> Result<(), ClientError> {
        let mut client = self.client.lock().unwrap();
        client.subscribe(format!("neolink/{}/#", self.name), QoS::AtMostOnce)?;
        Ok(())
    }

    fn update_status(&self) -> Result<(), ClientError> {
        self.send_message("status", "disconnected", true)?;
        Ok(())
    }

    pub fn send_message(&self, sub_topic: &str, message: &str, retain: bool) -> Result<(), ClientError> {
        let mut client = self.client.lock().unwrap();
        client.publish(
            format!("neolink/{}/{}", self.name, sub_topic),
            QoS::AtLeastOnce,
            retain,
            message,
        )?;
        Ok(())
    }

    pub fn get_message(&self) -> Result<Option<MqttReply>, RecvError> {
        let (_, receiver) = &self.incoming;
        let published_message = receiver.recv()?;
        Ok(Some(MqttReply {
            topic: published_message.topic,
            message: String::from_utf8_lossy(published_message.payload.as_ref()).into_owned(),
        }))
    }

    pub fn start(&self) -> Result<(), ClientError> {
        // This acts as an event loop
        let mut connection = self.connection.lock().unwrap();
        let (sender, _) = &self.incoming;
        info!("Starting MQTT Client for {}", self.name);
        loop {
            for (_i, notification) in connection.iter().enumerate() {
                trace!("MQTT Notification = {:?}", notification);
                if let Ok(notification) = notification {
                    match notification {
                        Event::Incoming(Incoming::ConnAck(connected)) => {
                            if ConnectReturnCode::Accepted == connected.code {
                                self.update_status()?;
                                // We succesfully logged in. Now ask for the cameras subscription.
                                self.subscribe()?;
                            }
                        }
                        Event::Incoming(Incoming::Publish(published_message)) => {
                            if sender.send(published_message).is_err() {
                                error!("Failed to publish motion message on mqtt");
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn publish_connection(&self, connected: bool) -> Result<(), ClientError> {
        match connected {
            true => self.send_message("status", "connected", true)?,
            false => self.send_message("status", "disconnected", true)?,
        }
        Ok(())
    }

    pub fn publish_motion(&self, movement_detected: bool) -> Result<(), ClientError> {
        match movement_detected {
            true => self.send_message("status/motion", "on", true)?,
            false => self.send_message("status/motion", "off", true)?,
        }
        Ok(())
    }
}
