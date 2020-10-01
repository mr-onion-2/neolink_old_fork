use super::MotionStatus;
use crossbeam_channel::{unbounded, Receiver, Sender};
use librumqttd::{Broker, Config, Error as BrokerError, LinkError, LinkTx};
use log::*;
use std::sync::Mutex;

const MAX_REQUESTS: usize = 1000000;

#[allow(dead_code)]
pub struct MQTT {
    link: Option<Mutex<LinkTx>>,
    broker: Option<Mutex<Broker>>,
}

#[allow(dead_code)]
pub struct MqttReply {
    topic: String,
    message: String,
}

impl MQTT {
    pub fn new(config: &Option<Config>) -> Self {
        let link;
        let broker;
        if let Some(mqtt_config) = &config {
            let broker_temp = Broker::new(mqtt_config.clone());

            let pubtx = broker_temp.link("localclient").unwrap();

            broker = Some(Mutex::new(broker_temp));
            link = Some(Mutex::new(pubtx));
        } else {
            broker = None;
            link = None;
        }

        Self { link, broker }
    }

    pub fn send_message(&self, topic: &str, message: &str) -> Result<(), LinkError> {
        if let Some(link) = &self.link {
            let mut link = link.lock().unwrap();
            link.connect(MAX_REQUESTS)?;
            let res = link.publish(topic, true, message);
            if res.is_err() {
                error!(
                    "Failed to publish on mqtt, topic: {}, message: {}",
                    topic, message
                );
            };
        }
        Ok(())
    }

    pub fn get_messages(&self, filter: &str) -> Result<Option<Vec<MqttReply>>, LinkError> {
        if let Some(link) = &self.link {
            let mut link = link.lock().unwrap();
            let mut rx = link.connect(MAX_REQUESTS)?;
            link.subscribe(filter)?;

            let reply = rx.recv()?;
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
        }
        Ok(None)
    }

    pub fn start(&self) -> Result<(), BrokerError> {
        // Mutex is permenantly locked now
        if let Some(broker) = &self.broker {
            info!("Starting MQTT Server");
            let mut broker = broker.lock().unwrap();
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
        match data {
            MotionStatus::MotionStart => {
                mqtt.send_message(&self.topic, "on")?;
            }
            MotionStatus::MotionStop => {
                mqtt.send_message(&self.topic, "off")?;
            }
            _ => {}
        }

        Ok(())
    }
}
