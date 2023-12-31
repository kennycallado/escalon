use tokio::sync::mpsc::Sender;

use crate::constants::MAX_CONNECTIONS;
use crate::types::message::{Action, Message};
use crate::Escalon;

impl Escalon {
    pub fn handle_action(&self) -> Sender<Message> {
        let escalon = self.clone();
        let (tx_handler, mut rx) = tokio::sync::mpsc::channel::<Message>(MAX_CONNECTIONS);

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg.action.clone() {
                    Action::Join(content) => {
                        if content.sender_id != escalon.id {
                            msg.handle_join(&escalon, content).await;
                        }
                    }

                    Action::Check(content) => {
                        if content.sender_id != escalon.id {
                            // msg.handle_check(&escalon, addr, content)
                            msg.handle_check(&escalon, content)
                        }
                    }

                    Action::FoundDead(content) => {
                        if content.sender_id != escalon.id {
                            msg.handle_found_dead(&escalon, content).await;
                        }
                    }

                    Action::TakeJobs(content) => {
                        if content.sender_id != escalon.id {
                            msg.handle_take_jobs(&escalon, content).await;
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

        tx_handler
    }
}
