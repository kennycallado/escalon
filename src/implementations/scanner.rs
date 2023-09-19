use anyhow::Result;
use chrono::Utc;
use std::net::SocketAddr;

use crate::constants::{HEARTBEAT_SECS, THRESHOLD_SECS};
use crate::types::client::Client;
use crate::types::message::Message;
use crate::Escalon;

impl Escalon {
    pub fn scanner_dead(&self) -> Result<()> {
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
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        escalon.redistribute_jobs(dead_id);
                    });
                }
            }
        });

        Ok(())
    }

    pub fn redistribute_jobs(&self, dead_id: String) {
        if self.should_skip_redistribution(&dead_id) {
            return;
        }

        let dead_client = self.remove_dead_client(&dead_id);

        match dead_client {
            None => eprintln!("Client with ID {} not found.", dead_id),
            Some(dead) => {
                let (n_jobs_dead, n_jobs_own, n_jobs_clients) =
                    self.calculate_job_counts(&dead);

                let n_jobs_total = n_jobs_clients + n_jobs_own + n_jobs_dead;

                let n_jobs_avg = self.calculate_average_jobs_per_client(n_jobs_total);

                // println!("Dead jobs: {}", n_jobs_dead);
                // println!("Own jobs: {}", n_jobs_own);
                // println!("Clients jobs: {}", n_jobs_clients);
                // println!("Average jobs per client: {}", n_jobs_avg);

                // println!("Total jobs: {}", n_jobs_total);
                // println!("Total clients: {}", n_clients);

                let mut clients_sorted = self.sort_clients_by_jobs(n_jobs_own);

                let mut n_jobs_to_redistribute = n_jobs_dead;
                let mut _n_jobs_redistributed = 0;
                let mut start_at = 1;
                let mut messages: Vec<(Message, SocketAddr)> = Vec::new();

                for (client_id, n_jobs, client_addr) in clients_sorted.iter_mut() {
                    if n_jobs_to_redistribute == 0 {
                        break;
                    }

                    let n_jobs_to_add =
                        self.calculate_jobs_to_add(*n_jobs, n_jobs_avg, n_jobs_to_redistribute);

                    n_jobs_to_redistribute -= n_jobs_to_add;
                    _n_jobs_redistributed += n_jobs_to_add;

                    self.process_job_redistribution(
                        &dead_id,
                        client_id,
                        client_addr,
                        n_jobs_to_add,
                        start_at,
                        &mut messages,
                    );

                    start_at += n_jobs_to_add;
                }

                self.spawn_job_redistribution_task(messages);

                println!("Redistribution complete.");
            }
        }
    }

    fn should_skip_redistribution(&self, dead_id: &str) -> bool {
        let clients = self.clients.lock().unwrap();
        if let Some(temp) = clients.get(dead_id) {
            if Utc::now().timestamp() - temp.last_seen < THRESHOLD_SECS {
                return true;
            }
        }
        false
    }

    fn remove_dead_client(&self, dead_id: &str) -> Option<Client> {
        let mut clients = self.clients.lock().unwrap();
        clients.remove(dead_id)
    }

    fn calculate_job_counts(&self, dead: &Client) -> (usize, usize, usize) {
        let n_jobs_dead = dead.state.jobs;
        let n_jobs_own = self.functions.count.as_ref()();
        let n_jobs_clients = self.calculate_total_jobs_in_clients();
        (n_jobs_dead, n_jobs_own, n_jobs_clients)
    }

    fn calculate_total_jobs_in_clients(&self) -> usize {
        self.clients.lock().unwrap().iter().fold(0, |acc, (_, client)| acc + client.state.jobs)
    }

    fn calculate_average_jobs_per_client(&self, n_jobs_total: usize) -> usize {
        let n_clients = self.clients.lock().unwrap().len() + 1;

        n_jobs_total / n_clients
    }

    fn sort_clients_by_jobs(&self, n_jobs_own: usize) -> Vec<(String, usize, SocketAddr)> {
        let mut clients_sorted;
        {
            clients_sorted = self
                .clients
                .lock()
                .unwrap()
                .iter()
                .map(|(id, client)| (id.clone(), client.state.jobs, client.address))
                .collect::<Vec<(String, usize, SocketAddr)>>();
        }

        clients_sorted.push((self.id.clone(), n_jobs_own, self.socket.local_addr().unwrap()));
        clients_sorted.sort_by(|(_, a, _), (_, b, _)| b.cmp(a).reverse());
        clients_sorted
    }

    fn calculate_jobs_to_add(
        &self,
        n_jobs: usize,
        n_jobs_avg: usize,
        n_jobs_to_redistribute: usize,
    ) -> usize {
        let mut n_jobs_to_add = n_jobs_avg - n_jobs;
        if n_jobs_to_add > n_jobs_to_redistribute {
            n_jobs_to_add = n_jobs_to_redistribute;
        }
        n_jobs_to_add
    }

    fn process_job_redistribution(
        &self,
        dead_id: &str,
        client_id: &str,
        address: &SocketAddr,
        n_jobs_to_add: usize,
        start_at: usize,
        messages: &mut Vec<(Message, SocketAddr)>,
    ) {
        if client_id == self.id {
            self.functions.add_from.clone().as_ref()(dead_id, start_at, n_jobs_to_add);
        } else {
            let message = Message::new_take_jobs(
                self.id.clone(),
                dead_id.to_string(),
                start_at,
                n_jobs_to_add,
            );
            messages.push((message, *address));
            self.update_distribution(dead_id, client_id, n_jobs_to_add);
        }
    }

    fn update_distribution(&self, dead_id: &str, client_id: &str, n_jobs_to_add: usize) {
        let mut distribution = self.distribution.lock().unwrap();
        distribution.push((client_id.to_string(), dead_id.to_string(), n_jobs_to_add, false));
    }

    fn spawn_job_redistribution_task(&self, messages: Vec<(Message, SocketAddr)>) {
        let tx_sender = self.tx_sender.clone();
        tokio::task::spawn(async move {
            for (message, addr) in messages {
                tx_sender.clone().unwrap().send((message, Some(addr))).await.unwrap();
            }
        });
    }
}
