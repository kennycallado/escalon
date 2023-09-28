use std::net::SocketAddr;

use chrono::Utc;

use crate::constants::THRESHOLD_SECS;
use crate::types::client::EscalonClient;
use crate::types::message::Message;
use crate::Escalon;

impl Escalon {
    pub async fn redistribute_jobs(&self, dead_id: String) {
        if self.should_skip_redistribution(&dead_id) {
            return;
        }

        let dead_client = self.remove_dead_client(&dead_id);

        match dead_client {
            None => eprintln!("Client with ID {} not found.", dead_id),
            Some(dead) => {
                let (n_jobs_dead, n_jobs_own, n_jobs_clients) =
                    self.calculate_job_counts_with_dead(&dead);

                let n_jobs_total = n_jobs_clients + n_jobs_own + n_jobs_dead;

                let n_jobs_avg = self.calculate_avg_jobs_client(n_jobs_total);

                // println!("Dead jobs: {}", n_jobs_dead);
                // println!("Own jobs: {}", n_jobs_own);
                // println!("Clients jobs: {}", n_jobs_clients);
                // println!("Average jobs per client: {}", n_jobs_avg);

                // println!("Total jobs: {}", n_jobs_total);
                // println!("Total clients: {}", n_clients);

                let mut clients_sorted = self.sort_clients_by_jobs(n_jobs_own);
                let mut n_jobs_to_redistribute = n_jobs_dead;
                let mut _n_jobs_redistributed = 0;
                // let mut start_at = 1;
                let mut start_at = 0;
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
                    )
                    .await;

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

    fn remove_dead_client(&self, dead_id: &str) -> Option<EscalonClient> {
        let mut clients = self.clients.lock().unwrap();
        clients.remove(dead_id)
    }

    fn calculate_job_counts_with_dead(&self, dead: &EscalonClient) -> (usize, usize, usize) {
        let n_jobs_dead = dead.state.jobs;
        let (n_jobs_own, n_jobs_clients) = self.calculate_job_counts();
        (n_jobs_dead, n_jobs_own, n_jobs_clients)
    }

    fn calculate_total_jobs_in_clients(&self) -> usize {
        self.clients.lock().unwrap().iter().fold(0, |acc, (_, client)| acc + client.state.jobs)
    }

    pub fn calculate_job_counts(&self) -> (usize, usize) {
        let n_jobs_own = self.manager.count();
        let n_jobs_clients = self.calculate_total_jobs_in_clients();
        (n_jobs_own, n_jobs_clients)
    }

    pub fn calculate_avg_jobs_client(&self, n_jobs_total: usize) -> usize {
        let n_clients = self.clients.lock().unwrap().len() + 1;

        n_jobs_total / n_clients
    }

    pub fn sort_clients_by_jobs(&self, n_jobs_own: usize) -> Vec<(String, usize, SocketAddr)> {
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

    pub fn calculate_jobs_to_add(
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

    fn update_distribution(
        &self,
        dead_id: &str,
        client_id: &str,
        start_at: usize,
        n_jobs_to_add: usize,
    ) {
        let mut distribution = self.distribution.lock().unwrap();
        distribution.push((
            client_id.to_string(),
            dead_id.to_string(),
            start_at,
            n_jobs_to_add,
            false,
        ));
    }

    pub async fn process_job_redistribution(
        &self,
        from_client: &str,
        client_id: &str,
        address: &SocketAddr,
        n_jobs_to_add: usize,
        start_at: usize,
        messages: &mut Vec<(Message, SocketAddr)>,
    ) {
        if client_id == self.id {
            let jobs = self.manager
                .take_jobs(from_client.to_string(), start_at, n_jobs_to_add)
                .await
                .unwrap();

            self.manager.drop_jobs(jobs).await.unwrap();
        } else {
            let message = Message::new_take_jobs(
                self.id.clone(),
                from_client.to_string(),
                start_at,
                n_jobs_to_add,
            );
            messages.push((message, *address));
            self.update_distribution(from_client, client_id, start_at, n_jobs_to_add);
        }
    }

    pub fn spawn_job_redistribution_task(&self, messages: Vec<(Message, SocketAddr)>) {
        let tx_sender = self.tx_sender.clone();
        tokio::task::spawn(async move {
            for (message, addr) in messages {
                tx_sender.clone().unwrap().send((message, Some(addr))).await.unwrap();
            }
        });
    }
}
