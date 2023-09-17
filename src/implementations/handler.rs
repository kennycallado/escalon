use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use anyhow::Result;
use chrono::Utc;

use crate::constants::MAX_CONNECTIONS;
use crate::types::message::{Action, Message};
use crate::Escalon;
use crate::{Client, ClientState};

#[rustfmt::skip]
// impl<J: IntoIterator
//         + Default
//         + Clone
//         + Debug
//         + for<'a> Deserialize<'a>
//         + Serialize
//         + Send
//         + Sync
//         + 'static > Escalon {
impl Escalon {
    pub fn handle_action(&self) -> Result<Sender<(Message, SocketAddr)>> {
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<(Message, SocketAddr)>(MAX_CONNECTIONS);

        // let clients = self.clients.clone();
        // let server_id = self.id.clone();
        // let server_start_time = self.start_time;
        // let tx_sender = self.tx_sender.clone();
        // let server_addr = self.addr.clone();

        let escalon = self.clone();

        tokio::task::spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                match msg.action {
                    Action::Join((id, start_time)) => {
                        if id != escalon.id {
                            if !escalon.clients.lock().unwrap().contains_key(&id) {
                                let message = Message {
                                    action: Action::Join((
                                        escalon.id.clone(),
                                        escalon.start_time,
                                    )),
                                };

                                escalon.tx_sender
                                    .as_ref()
                                    .unwrap()
                                    .send((message, Some(addr)))
                                    .await
                                    .unwrap();
                            }

                            insert(&mut escalon.clients.lock().unwrap(), id.clone(), start_time, addr);
                        }
                    }
                    Action::Check((id, jobs)) => {
                        if id != escalon.id {
                            update(&mut escalon.clients.lock().unwrap(), id.clone(), jobs);
                        }
                    },
                    // Action::UpdateDead((id, client)) => {
                    //     if id != escalon.id {
                    //         let addr = client.address;
                    //         let dead_id = escalon
                    //             .clients
                    //             .lock()
                    //             .unwrap()
                    //             .clone()
                    //             .into_iter()
                    //             .find(|(_, client)| client.address == addr)
                    //             .unwrap().0;

                    //         escalon.clients.lock().unwrap().remove(&dead_id);
                    //     }
                    // }
                }
            }

            #[rustfmt::skip]
            fn insert(clients: &mut HashMap<String, Client>, id: String, start_time: std::time::SystemTime, addr: SocketAddr) {
                clients
                    .entry(id)
                    .or_insert(Client {
                        start_time,
                        address: addr,
                        last_seen: Utc::now().timestamp(),
                        state: ClientState {
                            // memory: 0,
                            jobs: 0
                        },
                    });
            }

            #[rustfmt::skip]
            fn update(clients: &mut HashMap<String, Client>, id: String, jobs: usize) {
                clients
                    .entry(id)
                    .and_modify(|client| {
                        client.last_seen = Utc::now().timestamp();
                        client.state.jobs = jobs;
                    });
            }
        });

        Ok(tx)
    }
}
