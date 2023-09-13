use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::constants::MAX_CONNECTIONS;
use crate::types::message::{Action, Message};
use crate::Escalon;
use crate::{Client, ClientState};

#[rustfmt::skip]
impl<J: IntoIterator
        + Default
        + Clone
        + Debug
        + for<'a> Deserialize<'a>
        + Serialize
        + Send
        + Sync
        + 'static > Escalon<J> {
    pub fn handle_action(&self) -> Result<Sender<(Message<J>, SocketAddr)>> {
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<(Message<J>, SocketAddr)>(MAX_CONNECTIONS);

        let clients = self.clients.clone();
        let server_id = self.id.clone();
        let server_start_time = self.start_time;
        let tx_sender = self.tx_sender.clone();

        tokio::task::spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                match msg.action {
                    Action::Join((id, start_time)) => {
                        if id != server_id {
                            if !clients.lock().unwrap().contains_key(&id) {
                                let message = Message {
                                    action: Action::Join((
                                        server_id.clone(),
                                        server_start_time,
                                    )),
                                };

                                tx_sender
                                    .as_ref()
                                    .unwrap()
                                    .send((message, Some(addr)))
                                    .await
                                    .unwrap();
                            }

                            insert(&mut clients.lock().unwrap(), id.clone(), start_time, addr);
                        }
                    }
                    Action::Check((id, state)) => {
                        if id != server_id {
                            update(&mut clients.lock().unwrap(), id.clone(), state);
                        }
                    }
                }
            }

            #[rustfmt::skip]
            fn insert<J: Default>(clients: &mut HashMap<String, Client<J>>, id: String, start_time: std::time::SystemTime, addr: SocketAddr) {
                clients
                    .entry(id)
                    .or_insert(Client {
                        start_time,
                        address: addr,
                        last_seen: Utc::now().timestamp(),
                        state: ClientState { memory: 0, jobs: Default::default() },
                    });
            }

            #[rustfmt::skip]
            fn update<J>(clients: &mut HashMap<String, Client<J>>, id: String, state: ClientState<J>) {
                clients
                    .entry(id)
                    .and_modify(|client| {
                        client.last_seen = Utc::now().timestamp();
                        client.state = state;
                    });
            }
        });

        Ok(tx)
    }
}
