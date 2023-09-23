use anyhow::Result;
use std::net::SocketAddr;

use crate::types::message::Message;
use crate::Escalon;

impl Escalon {
    pub fn balancer(&self) -> Result<()> {
        let escalon = self.clone();

        tokio::spawn(async move {
            // Wait before starting the balancer
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                let (n_jobs_own, n_jobs_clients) = escalon.calculate_job_counts();
                let n_jobs_total = n_jobs_own + n_jobs_clients;

                let avg_jobs_per_client =
                    escalon.calculate_average_jobs_per_client(n_jobs_total);

                // println!("n_jobs_own: {}", n_jobs_own);
                // println!("n_jobs_clients: {}", n_jobs_clients);
                // println!("n_jobs_total: {}", n_jobs_total);
                // println!("avg_jobs_per_client: {}", avg_jobs_per_client);

                if n_jobs_own as f64 >= (avg_jobs_per_client as f64 * 1.5) {
                    let mut clientes_sorted = escalon.sort_clients_by_jobs(n_jobs_own);
                    let mut n_jobs_to_redistribute = n_jobs_own - avg_jobs_per_client;
                    let mut _n_jobs_redistributed = 0;
                    let mut start_at = 1;
                    let mut messages: Vec<(Message, SocketAddr)> = Vec::new();

                    for (client_id, n_jobs, client_addr) in clientes_sorted.iter_mut() {
                        if n_jobs_to_redistribute == 0 {
                            break;
                        }

                        let n_jobs_to_add = escalon.calculate_jobs_to_add(
                            *n_jobs,
                            avg_jobs_per_client,
                            n_jobs_to_redistribute,
                        );

                        n_jobs_to_redistribute -= n_jobs_to_add;
                        _n_jobs_redistributed += n_jobs_to_add;

                        escalon
                            .process_job_redistribution(
                                escalon.id.as_str(),
                                client_id,
                                client_addr,
                                n_jobs_to_add,
                                start_at,
                                &mut messages,
                            )
                            .await;

                        start_at += n_jobs_to_add;
                    }

                    escalon.spawn_job_redistribution_task(messages);
                }
            }
        });

        Ok(())
    }
}
