use crate::constants::HEARTBEAT_SECS;
use crate::types::message::Message;
use crate::Escalon;

impl Escalon {
    pub fn start_heartbeat(&self) {
        let escalon = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_SECS)).await;

                // send current state of the node
                let jobs = escalon.manager.count();
                let message = Message::new_check(escalon.id.clone(), jobs);

                escalon.tx_sender.as_ref().unwrap().send((message, None)).await.unwrap();
            }
        });
    }
}
