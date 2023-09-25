use chrono::Utc;

use crate::constants::{HEARTBEAT_SECS, THRESHOLD_SECS};
use crate::types::message::Message;
use crate::Escalon;

impl Escalon {
    pub fn scanner_dead(&self) {
        let escalon = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_SECS)).await;

                let dead_clients;
                {
                    dead_clients = escalon
                        .clients
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|(_, client)| {
                            Utc::now().timestamp() - client.last_seen > THRESHOLD_SECS
                        })
                        .map(|(id, _)| id.clone())
                        .collect::<Vec<String>>();
                }

                for dead_id in dead_clients {
                    println!("Dead client: {}", dead_id);

                    // send UpdateDead to all
                    let message = Message::new_found_dead(escalon.id.clone(), dead_id.clone());
                    escalon.tx_sender.clone().unwrap().send((message, None)).await.unwrap();

                    let escalon = escalon.clone();
                    tokio::spawn(async move {
                        // wait to avoid clients colitions
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        escalon.redistribute_jobs(dead_id).await;
                    });
                }
            }
        });
    }
}
