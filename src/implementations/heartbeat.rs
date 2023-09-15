use std::fmt::Debug;

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::constants::{HEARTBEAT_SECS, THRESHOLD_SECS};
use crate::types::client::ClientState;
use crate::types::message::{Action, Message};
use crate::Escalon;

#[rustfmt::skip]
impl<J: IntoIterator
        + Default
        + Clone
        + Debug
        + for<'a> Deserialize<'a>
        + Serialize
        + Send
        + Sync
        + 'static > Escalon<J> {
    pub fn start_heartbeat(&self) -> Result<()> {
        let clients = self.clients.clone();
        let server_id = self.id.clone();
        let tx_sender = self.tx_sender.clone();
        let own_state = self.own_state.clone();

        tokio::task::spawn(async move {
            // needed for procfs
            // let process = procfs::process::Process::myself().unwrap();

            loop {
                // sleeps
                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_SECS)).await;

                // update own state
                let jobs = own_state.lock().unwrap().jobs.lock().unwrap().clone();

                //
                // get memory stats
                //
                // let memroy_stats: usize = memory_stats::memory_stats().unwrap().physical_mem / 1024;
                // let procfs: u64 = process.status().unwrap().vmrss.unwrap();
                // let memory_procinfo = procinfo::pid::statm(std::process::id().try_into().unwrap())
                //     .unwrap()
                //     .resident;
                // own_state.lock().unwrap().memory = memory;
                //

                let own_state = ClientState {
                    // memory,
                    jobs,
                };

                let message = Message {
                    action: Action::Check((server_id.clone(), own_state)),
                };

                tx_sender.as_ref().unwrap().send((message, None)).await.unwrap();

                // update clients
                let mut clients = clients.lock().unwrap();
                let now = Utc::now().timestamp();

                // detect dead clients
                // were update on handle_action
                let dead_clients = clients
                    .iter()
                    .filter(|(_, client)| now - client.last_seen > THRESHOLD_SECS)
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<String>>();

                for id in dead_clients {
                    let dead = clients.get(&id).unwrap().clone();
                    println!("{:?} is dead", dead);

                    let num_jobs = dead.state.jobs.into_iter().count();
                    println!("{} jobs from {} were lost", num_jobs, id);

                    clients.remove(&id);
                }
            }
        });

        Ok(())
    }
}
