use anyhow::Result;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;

use crate::constants::MAX_CONNECTIONS;
use crate::types::message::{Action, Message};
use crate::Escalon;

impl Escalon {
    pub fn handle_action(&self) -> Result<Sender<(Message, SocketAddr)>> {
        let escalon = self.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Message, SocketAddr)>(MAX_CONNECTIONS);

        tokio::spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                match msg.action.clone() {
                    Action::Join(content) => {
                        if content.sender_id != escalon.id {
                            msg.handle_join(&escalon, addr, content).await;
                        }
                    }

                    Action::Check(content) => {
                        if content.sender_id != escalon.id {
                            msg.handle_check(&escalon, content)
                        }
                    }

                    Action::FoundDead(content) => {
                        if content.sender_id != escalon.id {
                            msg.handle_found_dead(&escalon, addr, content).await;
                        } else {
                            let message =
                                Message::new_join(escalon.id.clone(), escalon.start_time);

                            escalon
                                .tx_sender
                                .as_ref()
                                .unwrap()
                                .send((message, None))
                                .await
                                .unwrap();
                        }
                    }

                    Action::TakeJobs(content) => {
                        if content.sender_id != escalon.id {
                            msg.handle_take_jobs(&escalon, addr, content).await;
                        }
                    }

                    Action::Done(content) => {
                        if content.sender_id != escalon.id {
                            msg.handle_done(&escalon, content).await;
                        }
                    }
                }
            }
        });

        Ok(tx)
    }
}
