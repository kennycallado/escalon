use anyhow::Result;
use chrono::Utc;

use crate::constants::{HEARTBEAT_SECS, THRESHOLD_SECS};
use crate::types::message::{Action, Message};
use crate::Escalon;

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
    pub fn start_heartbeat(&self) -> Result<()> {
        let escalon = self.clone();

        tokio::task::spawn(async move {
            let escalon = escalon;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_SECS)).await;

                // send current state of the node
                let jobs = escalon.own_state.as_ref();

                let message = Message {
                    action: Action::Check((escalon.id.clone(), jobs())),
                };
                escalon.tx_sender.as_ref().unwrap().send((message, None)).await.unwrap();

                // detect dead clients
                let dead_clients = escalon.clients.lock().unwrap()
                    .iter()
                    .filter(|(_, client)| Utc::now().timestamp() - client.last_seen > THRESHOLD_SECS)
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<String>>();

                for id in dead_clients {
                    let dead = escalon.clients.lock().unwrap().get(&id).unwrap().clone();

                    // // send UpdateDead to all
                    // let message = Message {
                    //     action: Action::UpdateDead((escalon.id.clone(), dead)),
                    // };
                    // escalon.tx_sender.as_ref().unwrap().send((message, None)).await.unwrap();

                    // escalon.redistribute_jobs(id.clone()).await;
                }
            }
        });

        Ok(())
    }

    pub async fn redistribute_jobs(&self, id: String) {
        // Wait for other nodes to inform about the dead client.
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Lock the clients mutex once.
        let dead;
        {
            dead = self.clients.lock().unwrap().remove(&id);

        }

        let clients = self.clients.lock().unwrap();

        if let Some(dead) = dead {

            // Extract the dead client's state.
            let n_jobs_dead = dead.state.jobs;
            let n_jobs_own = self.own_state.as_ref()();
            let n_jobs_clients = clients
                .iter()
                .fold(0, |acc, (_, client)| { acc + client.state.jobs });

            let n_jobs_total = n_jobs_clients + n_jobs_own + n_jobs_dead;

            // Calculate the average number of jobs per client.
            let n_clients = clients.len() + 1; // Add 1 for the current client.
            let n_jobs_avg = n_jobs_total / n_clients;

            println!("Dead jobs: {}", n_jobs_dead);
            println!("Own jobs: {}", n_jobs_own);
            println!("Total jobs: {}", n_jobs_clients);
            println!("Average jobs per client: {}", n_jobs_avg);

            println!("Total clients: {}", n_clients); // includes the current client

        } else {
            println!("Client with ID {} not found.", id);
        }
    }
}
