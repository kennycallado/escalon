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

                for dead_id in dead_clients {
                    // let dead = escalon.clients.lock().unwrap().get(&dead_id).unwrap().clone();

                    // send UpdateDead to all
                    let message = Message {
                        action: Action::FoundDead((escalon.id.clone(), dead_id.clone())),
                    };
                    escalon.tx_sender.as_ref().unwrap().send((message, None)).await.unwrap();

                    escalon.redistribute_jobs(dead_id).await;
                }
            }
        });

        Ok(())
    }

    pub async fn redistribute_jobs(&self, id: String) {
        // Wait for other nodes to inform about the dead client.
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

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

            if n_clients == 1 {
                println!("All jobs to self.");

                return ;
            }

            // create a vector of clients sorted by number of jobs and include self
            let mut clients_sorted = clients
                .iter()
                .map(|(id, client)| (id.clone(), client.state.jobs))
                .collect::<Vec<(String, usize)>>();
            clients_sorted.push((self.id.clone(), n_jobs_own));
            clients_sorted.sort_by(|(_, a), (_, b)| b.cmp(a).reverse());

            // redistribute jobs from dead to clients with less jobs
            // and print a message for each client with the number
            // of jobs to add and where to start

            let mut n_jobs_to_redistribute = n_jobs_dead;
            let mut _n_jobs_redistributed = 0;
            let mut n_jobs_to_add;
            let mut next = 1;

            for (id, n_jobs) in clients_sorted {
                if n_jobs_to_redistribute == 0 {
                    break;
                }

                n_jobs_to_add = n_jobs_avg - n_jobs;

                if n_jobs_to_add > n_jobs_to_redistribute {
                    n_jobs_to_add = n_jobs_to_redistribute;
                }

                n_jobs_to_redistribute -= n_jobs_to_add;
                _n_jobs_redistributed += n_jobs_to_add;

                if id == self.id { println!("Self"); }
                println!("{}: {} jobs to add, starting at {}.", id, n_jobs_to_add, next);
                next += n_jobs_to_add;
            }

            println!("Redistribution complete.");
        } else {
            println!("Client with ID {} not found.", id);
        }
    }
}
