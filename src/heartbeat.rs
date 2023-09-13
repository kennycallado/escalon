use anyhow::Result;

use chrono::Utc;

use crate::Escalon;
use crate::constants::{HEARTBEAT_SECS, THRESHOLD_SECS};
use crate::message::{Message, Action};

impl<J: Send + Sync + 'static> Escalon<J> {
    pub fn start_heartbeat(&self) -> Result<()> {
        let clients = self.clients.clone();
        let server_count = self.jobs.clone();
        let server_id = self.id.clone();
        let tx_sender = self.tx_sender.clone();
        let own_state = self.own_state.clone();

        tokio::task::spawn(async move {
            loop {
                // sleeps
                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_SECS)).await;

                // update own state
                own_state.lock().unwrap().memory =
                    procinfo::pid::statm(std::process::id().try_into().unwrap())
                        .unwrap()
                        .resident;
                own_state.lock().unwrap().jobs = server_count.lock().unwrap().len();

                let own_state = own_state.lock().unwrap().to_owned();

                // send heartbeat
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

                    clients.remove(&id);
                }
            }
        });

        Ok(())
    }
}
