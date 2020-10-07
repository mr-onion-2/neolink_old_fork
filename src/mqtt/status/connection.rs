use crossbeam_channel::{unbounded, Receiver, Sender};
use log::*;
use std::sync::{Arc};
use crate::mqtt::MQTT;

pub struct ConnectionSender {
    topic: String,
    receiver: Receiver<ConnectionStatus>,
    mqtt: Arc<MQTT>,
}

pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

impl<'a> ConnectionSender {
    pub fn create_tx(mqtt: Arc<MQTT>) -> Sender<ConnectionStatus> {
        let (sender, receiver) = unbounded::<ConnectionStatus>();
        let me = Self {
            topic: "status".to_string(),
            receiver,
            mqtt,
        };
        std::thread::spawn(move || loop {
            me.poll_status();
        });
        sender
    }

    fn poll_status(&self) {
        let data = self.receiver.recv().expect("We should get something");
        trace!("Got connection status");
        match data {
            ConnectionStatus::Connected => {
                if (*self.mqtt).send_message(&self.topic, "connected", true).is_err() {
                    error!("Failed to send connection status to mqtt");
                }
            }
            ConnectionStatus::Disconnected => {
                if (*self.mqtt).send_message(&self.topic, "disconnected", true).is_err() {
                    error!("Failed to send connection status to mqtt");
                }
            }
        }
        trace!("Finished posting connection status status");
    }
}
