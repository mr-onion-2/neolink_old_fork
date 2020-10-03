// Dummy version to disable it when compiled on msvc
use crate::MotionStatus;
use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::Deserialize;
use log::*;

#[allow(dead_code)]
pub struct MotionWriter {
    topic: String,
    receiver: Receiver<MotionStatus>,
}

#[allow(dead_code)]
pub struct MqttReply {
    topic: String,
    message: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MqttConfig {

}

#[allow(dead_code)]
#[allow(unused_variables)]
impl<'a> MotionWriter {
    pub fn new_with_tx(cam_name: &str) -> (Self, Sender<MotionStatus>) {
        let (sender, receiver) = unbounded::<MotionStatus>();
        let me = Self {
            topic: format!("/neolink/{}/status/motion", cam_name),
            receiver,
        };
        (me, sender)
    }

    pub fn poll_status(&self, mqtt: &MQTT) -> Result<(), ()> {
        Ok(())
    }
}

#[allow(dead_code)]
pub struct MQTT;

#[allow(dead_code)]
#[allow(unused_variables)]
impl MQTT {
    pub fn new(config: &Option<MqttConfig>) -> Self {
        if config.is_some() {
            warn!("MQTT is disabled on msvc build, until it is fixed upstream.");
        }
        Self {}
    }

    fn ensure_connected(&self) {}

    pub fn send_message(&self, topic: &str, message: &str) -> Result<(), ()> {
        Ok(())
    }

    pub fn get_messages(&self) -> Result<Option<Vec<MqttReply>>, ()> {
        Ok(None)
    }

    pub fn start(&self) -> Result<(), ()> {
        Ok(())
    }
}
