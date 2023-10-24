use std::net::SocketAddr;

use crate::types::message::Message;
use crate::{Distrib, Escalon};

impl Escalon {
    pub fn balancer(&self) {
        let escalon = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

                let (n_jobs_own, n_jobs_clients) = escalon.calculate_job_counts();
                let n_jobs_total = n_jobs_own + n_jobs_clients;
                let avg_jobs_client = escalon.calculate_avg_jobs_client(n_jobs_total);

                if n_jobs_own as f64 >= (avg_jobs_client as f64 * 1.1)
                    && (n_jobs_own - avg_jobs_client) > 3
                {
                    let mut clients_sorted = escalon.sort_clients_by_jobs(n_jobs_own);
                    let mut n_jobs_to_redistribute = n_jobs_own - avg_jobs_client;
                    let mut _n_jobs_redistributed = 0;
                    // let mut start_at = 1;
                    let mut start_at = 0;
                    let mut messages: Vec<(Message, SocketAddr)> = Vec::new();

                    for (client_id, n_jobs, client_addr) in clients_sorted.iter_mut() {
                        if n_jobs_to_redistribute == 0 {
                            break;
                        }

                        let n_jobs = escalon.calculate_jobs_to_add(
                            *n_jobs,
                            avg_jobs_client,
                            n_jobs_to_redistribute,
                        );

                        n_jobs_to_redistribute -= n_jobs;
                        _n_jobs_redistributed += n_jobs;

                        let distrib = Distrib {
                            client_id: client_id.to_string(),
                            take_from: escalon.id.clone(),
                            start_at,
                            n_jobs,
                            done: false,
                        };

                        escalon
                            .process_job_redistribution(distrib, client_addr, &mut messages)
                            .await;

                        start_at += n_jobs;
                    }

                    escalon.spawn_job_redistribution_task(messages);
                }
            }
        });
    }
}
