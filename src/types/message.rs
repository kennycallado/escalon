use std::net::SocketAddr;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::Escalon;
use crate::{ClientState, EscalonClient};

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
    pub address: SocketAddr,
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
    pub take_from: String,
    pub start_at: usize,
    pub n_jobs: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DoneContent {
    pub sender_id: String,
    pub take_from: String,
    pub n_jobs: Vec<String>,
}

impl Message {
    pub fn new_join(
        addr: SocketAddr,
        id: impl Into<String>,
        start_time: std::time::SystemTime,
    ) -> Self {
        Self {
            action: Action::Join(JoinContent {
                address: addr,
                sender_id: id.into(),
                start_time,
            }),
        }
    }

    // TODO
    // quizá siempre enviar el join pero comprobar antes de insertar
    pub async fn handle_join(&self, escalon: &Escalon, content: JoinContent) {
        if !escalon.clients.lock().unwrap().contains_key(&content.sender_id) {
            let message = Message::new_join(escalon.address, &escalon.id, escalon.start_time);

            escalon
                .tx_sender
                .clone()
                .unwrap()
                .send((message, Some(content.address)))
                .await
                .unwrap();
        }

        escalon.clients.lock().unwrap().insert(
            content.sender_id,
            EscalonClient {
                start_time: content.start_time,
                address: content.address,
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

    // TODO
    // quizá enviar un join si no está en la lista de clientes
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

    // TODO
    // quizá spawn un thread para enviar el join y quitar async de la firma
    pub async fn handle_found_dead(&self, escalon: &Escalon, content: FoundDeadContent) {
        // si soy yo enviar un join 
        if content.dead_id == escalon.id {
            let message = Message::new_join(escalon.address, &escalon.id, escalon.start_time);
            escalon.tx_sender.clone().unwrap().send((message, None)).await.unwrap();
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
        id: impl Into<String>,
        take_from: impl Into<String>,
        start_at: usize,
        jobs: usize,
    ) -> Self {
        Self {
            action: Action::TakeJobs(TakeJobsContent {
                sender_id: id.into(),
                take_from: take_from.into(),
                start_at,
                n_jobs: jobs,
            }),
        }
    }

    pub async fn handle_take_jobs(&self, escalon: &Escalon, content: TakeJobsContent) {
        let done = escalon
            .manager
            .take_jobs(content.take_from.clone(), content.start_at, content.n_jobs)
            .await
            .unwrap();

        let clients = escalon.clients.lock().unwrap().clone();
        let sender = clients.get(&content.sender_id).unwrap();
        let chunks = done.chunks(50);
        for chunk in chunks {
            let message = Message::new_done(
                escalon.id.clone(),
                content.take_from.clone(),
                chunk.to_vec(),
            );

            escalon
                .tx_sender
                .clone()
                .unwrap()
                .send((message, Some(sender.address)))
                .await
                .unwrap();
        }
    }
}

impl Message {
    pub fn new_done(
        id: impl Into<String>,
        take_from: impl Into<String>,
        jobs: Vec<String>,
    ) -> Self {
        Self {
            action: Action::Done(DoneContent {
                sender_id: id.into(),
                take_from: take_from.into(),
                n_jobs: jobs,
            }),
        }
    }

    pub async fn handle_done(&self, escalon: &Escalon, content: DoneContent) {
        // Should manage own jobs and case dead client jobs
        escalon.manager.drop_jobs(content.n_jobs).await.unwrap();

        let mut temp = escalon.distribution.lock().unwrap();
        // temp.retain(|(sender, form_client, _start_at, _n_jobs, _done)| {
        temp.retain(|distrib| {
            !(*distrib.client_id == content.sender_id
                && *distrib.take_from == content.take_from)
        });
    }
}
