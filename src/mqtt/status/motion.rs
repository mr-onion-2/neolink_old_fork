use crate::{MotionStatus};
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::*;
use std::sync::{Arc};
use crate::mqtt::MQTT;

pub struct MotionWriter {
    topic: String,
    receiver: Receiver<MotionStatus>,
    mqtt: Arc<MQTT>,
}

impl<'a> MotionWriter {
    pub fn create_tx(mqtt: Arc<MQTT>) -> Sender<MotionStatus> {
        let (sender, receiver) = unbounded::<MotionStatus>();
        let me = Self {
            topic: "status/motion".to_string(),
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
        trace!("Got motion status");
        match data {
            MotionStatus::MotionStart => {
                if (*self.mqtt).send_message(&self.topic, "on", true).is_err() {
                    error!("Failed to send motion to mqtt");
                }
            }
            MotionStatus::MotionStop => {
                if (*self.mqtt).send_message(&self.topic, "off", true).is_err() {
                    error!("Failed to send motion to mqtt");
                }
            }
            _ => {
                trace!("Motion status was no change");
            }
        }
        trace!("Finished posting motion status");
    }
}
