use crate::MotionStatus;
use crossbeam_channel::{unbounded, Receiver, Sender};
pub use librumqttd::Config as MqttConfig;
use librumqttd::{Broker, Error as BrokerError, LinkError, LinkRx, LinkTx};
use log::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const MAX_REQUESTS: usize = 200;

#[allow(dead_code)]
#[cfg(not(target_env = "msvc"))]
pub struct MQTT {
    broker: Mutex<Option<Broker>>,
    tx: Mutex<Option<LinkTx>>,
    rx: Mutex<Option<LinkRx>>,
    connected: Mutex<AtomicBool>,
}

#[allow(dead_code)]
pub struct MqttReply {
    topic: String,
    message: String,
}

impl MQTT {
    pub fn new(config: &Option<MqttConfig>) -> Self {
        let broker;
        let tx;
        let rx;
        if let Some(mqtt_config) = &config {
            let broker_temp = Broker::new(mqtt_config.clone());
            let tx_temp = broker_temp.link("localclient").unwrap();

            broker = Mutex::new(Some(broker_temp));
            tx = Mutex::new(Some(tx_temp));
            rx = Mutex::new(None); // Connect fails until server is up so we init this as None
        } else {
            broker = Mutex::new(None);
            tx = Mutex::new(None);
            rx = Mutex::new(None);
        }

        Self {
            broker,
            tx,
            rx,
            connected: Mutex::new(AtomicBool::new(false)),
        }
    }

    fn ensure_connected(&self) {
        let connected = &mut self.connected.lock().unwrap();
        if !connected.load(Ordering::Acquire) {
            let tx = &mut self.tx.lock().unwrap();
            if let Some(tx) = tx.as_mut() {
                let rx_locked = &mut self.rx.lock().unwrap();
                rx_locked.get_or_insert_with(|| tx.connect(MAX_REQUESTS).unwrap());
                let _ = tx.subscribe("#");
                connected.store(true, Ordering::Release);
            }
        }
    }

    pub fn send_message(&self, topic: &str, message: &str) -> Result<(), LinkError> {
        self.ensure_connected();
        let tx = &mut self.tx.lock().unwrap();
        if let Some(tx) = tx.as_mut() {
            let res = tx.publish(topic, false, message);
            if res.is_err() {
                error!(
                    "Failed to publish on mqtt, topic: {}, message: {}",
                    topic, message
                );
            };
        }
        Ok(())
    }

    pub fn get_messages(&self) -> Result<Option<Vec<MqttReply>>, LinkError> {
        self.ensure_connected();

        let reply;
        {
            // Unlock as soon as possible
            let rx_locked = &mut self.rx.lock().unwrap();
            let rx = rx_locked.as_mut().unwrap();

            reply = rx.recv()?;
        }
        if let Some(message) = reply {
            debug!(
                "Incoming. Topic = {}, Payload = {:?}",
                message.topic, message.payload
            );
            let mut replies = vec![];
            for payload in message.payload {
                replies.push(MqttReply {
                    topic: message.topic.clone(),
                    message: String::from_utf8_lossy(payload.as_ref()).to_string(),
                })
            }
            return Ok(Some(replies));
        }
        Ok(None)
    }

    pub fn start(&self) -> Result<(), BrokerError> {
        let broker = &mut self.broker.lock().unwrap();
        if let Some(broker) = broker.as_mut() {
            info!("Starting MQTT Server");
            // Mutex is permenantly locked now since this loops forever
            broker.start()?;
        }
        Ok(())
    }
}

pub struct MotionWriter {
    topic: String,
    receiver: Receiver<MotionStatus>,
}

impl<'a> MotionWriter {
    pub fn new_with_tx(cam_name: &str) -> (Self, Sender<MotionStatus>) {
        let (sender, receiver) = unbounded::<MotionStatus>();
        let me = Self {
            topic: format!("/neolink/{}/status/motion", cam_name),
            receiver,
        };
        (me, sender)
    }

    pub fn poll_status(&self, mqtt: &MQTT) -> Result<(), LinkError> {
        let data = self.receiver.recv().expect("We should get something");
        debug!("Got motion status");
        match data {
            MotionStatus::MotionStart => {
                if mqtt.send_message(&self.topic, "on").is_err() {
                    error!("Failed to send motion to mqtt");
                }
            }
            MotionStatus::MotionStop => {
                if mqtt.send_message(&self.topic, "off").is_err() {
                    error!("Failed to send motion to mqtt");
                }
            }
            _ => {
                trace!("Motion status was no change");
            }
        }
        debug!("Finished posting motion status");
        Ok(())
    }
}
