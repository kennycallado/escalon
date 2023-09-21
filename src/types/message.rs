use std::net::SocketAddr;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{Client, ClientState};
use crate::{Distrib, Escalon};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub action: Action,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Action {
    Join(JoinContent),
    Check(CheckContent),
    FoundDead(FoundDeadContent), // for now just the id
    TakeJobs(TakeJobsContent),
    Done(DoneContent),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct JoinContent {
    pub sender_id: String,
    pub start_time: std::time::SystemTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CheckContent {
    pub sender_id: String,
    pub jobs: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FoundDeadContent {
    pub sender_id: String,
    pub dead_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TakeJobsContent {
    pub sender_id: String,
    pub from_client: String,
    pub start_at: usize,
    pub jobs: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DoneContent {
    pub sender_id: String,
    pub from_client: String,
}

impl Message {
    pub fn new_join(id: String, start_time: std::time::SystemTime) -> Self {
        Self {
            action: Action::Join(JoinContent {
                sender_id: id,
                start_time,
            }),
        }
    }

    pub async fn handle_join(&self, escalon: &Escalon, addr: SocketAddr, content: JoinContent) {
        if !escalon.clients.lock().unwrap().contains_key(&content.sender_id) {
            let message = Message::new_join(escalon.id.clone(), escalon.start_time);
            escalon.tx_sender.clone().unwrap().send((message, Some(addr))).await.unwrap();
        }

        escalon.clients.lock().unwrap().insert(
            content.sender_id,
            Client {
                start_time: content.start_time,
                address: addr,
                last_seen: Utc::now().timestamp(),
                state: ClientState {
                    jobs: 0,
                },
            },
        );
    }
}

impl Message {
    pub fn new_check(id: String, jobs: usize) -> Self {
        Self {
            action: Action::Check(CheckContent {
                sender_id: id,
                jobs,
            }),
        }
    }

    pub fn handle_check(&self, escalon: &Escalon, content: CheckContent) {
        escalon.clients.lock().unwrap().entry(content.sender_id).and_modify(|client| {
            client.last_seen = Utc::now().timestamp();
            client.state.jobs = content.jobs;
        });
    }
}

impl Message {
    pub fn new_found_dead(id: String, dead_id: String) -> Self {
        Self {
            action: Action::FoundDead(FoundDeadContent {
                sender_id: id,
                dead_id,
            }),
        }
    }

    pub async fn handle_found_dead(
        &self,
        escalon: &Escalon,
        addr: SocketAddr,
        content: FoundDeadContent,
    ) {
        if content.dead_id == escalon.id {
            let message = Message::new_join(escalon.id.clone(), escalon.start_time);
            escalon.tx_sender.clone().unwrap().send((message, Some(addr))).await.unwrap();
        }

        let clients = match escalon.clients.lock() {
            Ok(clients) => clients.clone(),
            Err(_) => {
                eprintln!("Error getting clients");

                return;
            }
        };

        if let Some(sender) = clients.get(&content.sender_id) {
            if sender.start_time < escalon.start_time {
                escalon.clients.lock().unwrap().remove(&content.dead_id);
            }
        };
    }
}

impl Message {
    pub fn new_take_jobs(
        id: String,
        from_client: String,
        start_at: usize,
        jobs: usize,
    ) -> Self {
        Self {
            action: Action::TakeJobs(TakeJobsContent {
                sender_id: id,
                from_client,
                start_at,
                jobs,
            }),
        }
    }

    pub async fn handle_take_jobs(
        &self,
        escalon: &Escalon,
        addr: SocketAddr,
        content: TakeJobsContent,
    ) {
        escalon.functions.add_from.as_ref()(
            content.from_client.clone(),
            content.start_at,
            content.jobs,
        );

        let message = Message::new_done(escalon.id.clone(), content.from_client.clone());
        escalon.tx_sender.clone().unwrap().send((message, Some(addr))).await.unwrap();
    }
}

impl Message {
    pub fn new_done(id: String, dead_id: String) -> Self {
        Self {
            action: Action::Done(DoneContent {
                sender_id: id,
                from_client: dead_id.clone(),
            }),
        }
    }

    pub fn handle_done(&self, escalon: &Escalon, content: DoneContent) {
        if content.from_client == escalon.id {
            let temp = escalon.distribution.lock().unwrap();
            let info: &Distrib = temp
                .iter()
                .find(|(sender, from_client, _, _, _)| {
                    *sender == content.sender_id && *from_client == escalon.id
                })
                .unwrap();

            // call function to remove
            println!("Removing {} jobs from {}", info.3, info.1);
        }

        // let dist;
        {
            let mut temp = escalon.distribution.lock().unwrap();
            temp.retain(|(sender, form_client, _, _, _)| {
                !(*sender == content.sender_id && *form_client == content.from_client)
            });

            // dist = temp;
        }

        // if dist.len() == 0 {
        //     println!("All distributed");
        // }
    }
}
