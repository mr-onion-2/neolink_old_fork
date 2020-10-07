use crossbeam_channel::{unbounded, Receiver, Sender};
use log::*;
use std::sync::{Arc};
use crate::mqtt::MQTT;

pub struct LedReader {
    topic: String,
    sender: Sender<LedStatus>,
    mqtt: Arc<MQTT>,
}

pub enum LedStatus {
    On,
    Off,
}

impl LedReader{
    pub fn create_rx(mqtt: Arc<MQTT>) -> Receiver<LedStatus> {
        let (sender, receiver) = unbounded::<LedStatus>();
        let me = Self {
            topic: "status".to_string(),
            sender,
            mqtt,
        };
        std::thread::spawn(move || loop {
            //me.poll_status();
        });
        receiver
    }
}
