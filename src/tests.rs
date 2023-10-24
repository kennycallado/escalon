use anyhow::Result;

use super::*;

use crate::constants::MAX_CONNECTIONS;
use crate::types::message::{Action, JoinContent};

#[derive(Clone)]
struct Manager {
    jobs: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl EscalonTrait for Manager {
    fn count(&self) -> usize {
        let count = self.jobs.lock().unwrap().len();
        count
    }

    async fn take_jobs(
        &self,
        from: String,
        start_at: usize,
        count: usize,
    ) -> Result<Vec<String>, ()> {
        println!("{}: {} {}", from, start_at, count);
        Ok(Vec::new())
    }

    async fn drop_jobs(&self, jobs: Vec<String>) -> Result<(), ()> {
        println!("{:?}", jobs);
        Ok(())
    }
}

#[tokio::test]
async fn test_server_creation_and_listen() -> Result<()> {
    let manager = Manager {
        jobs: Arc::new(Mutex::new(vec![1, 2, 3])),
    };

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_svc("255.255.255.255".parse().unwrap())
        .set_port(0)
        .set_manager(manager)
        .build()
        .await;

    server.listen().await;
    // assert!(server.listen().await.is_ok());

    drop(server);

    Ok(())
}

#[tokio::test]
#[should_panic]
async fn test_bind_twice() {
    let manager = Manager {
        jobs: Arc::new(Mutex::new(vec![1, 2, 3])),
    };

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_svc("255.255.255.255".parse().unwrap())
        .set_port(0) // Use a random available port
        .set_manager(manager)
        .build()
        .await;

    server.listen().await;
    // assert!(server.listen().await.is_ok());
    tokio::net::UdpSocket::bind(server.socket.local_addr().unwrap()).await.unwrap();

    drop(server);
}

#[tokio::test]
#[should_panic]
async fn test_server_invalid_port() {
    let manager = Manager {
        jobs: Arc::new(Mutex::new(vec![1, 2, 3])),
    };

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_svc("255.255.255.255".parse().unwrap())
        .set_port(1)
        .set_manager(manager)
        .build()
        .await;

    server.listen().await;
    // assert!(server.listen().await.is_ok());

    drop(server);
}

#[tokio::test]
async fn test_intercept_before_send_join() -> Result<()> {
    let manager = Manager {
        jobs: Arc::new(Mutex::new(vec![1, 2, 3])),
    };

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_svc("255.255.255.255".parse().unwrap())
        .set_port(0) // Use a random available port
        .set_manager(manager)
        .build()
        .await;

    let (tx_sender, mut rx_sender) =
        tokio::sync::mpsc::channel::<(Message, Option<SocketAddr>)>(MAX_CONNECTIONS);
    server.tx_sender = Some(tx_sender);

    server.listen().await;
    // assert!(server.send_join().is_ok());

    let received_message: (Message, Option<SocketAddr>) = rx_sender.recv().await.unwrap();

    let id = server.id;
    let start_time = server.start_time;
    assert_eq!(received_message.1, None);
    assert_eq!(
        received_message.0.action,
        Action::Join(JoinContent {
            sender_id: id,
            address: server.address,
            start_time
        })
    );

    Ok(())
}

#[tokio::test]
async fn test_intercept_before_hertbeat() -> Result<()> {
    let manager = Manager {
        jobs: Arc::new(Mutex::new(vec![1, 2, 3])),
    };

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_svc("255.255.255.255".parse().unwrap())
        .set_port(0)
        .set_manager(manager)
        .build()
        .await;

    let (tx_sender, mut rx_sender) =
        tokio::sync::mpsc::channel::<(Message, Option<SocketAddr>)>(MAX_CONNECTIONS);
    server.tx_sender = Some(tx_sender);

    server.listen().await;
    // assert!(server.start_heartbeat().is_ok());

    let received_message: (Message, Option<SocketAddr>) = rx_sender.recv().await.unwrap();

    assert_eq!(received_message.1, None);
    assert!(matches!(received_message.0.action, Action::Check(..)));

    Ok(())
}

// IT DOESN'T WORK
// #[tokio::test(flavor = "multi_thread")]
// #[tokio::test]
// async fn test_intercept_after_send_join() -> Result<()> {

//     // cargo test -- --nocapture
//     // cargo watch "test -- --nocapture"

// let manager = Manager {
//     jobs: Arc::new(Mutex::new(vec![1, 2, 3])),
// };

//     let mut server = Escalon::new()
//         .set_id("test")
//         .set_addr("127.0.0.1".parse().unwrap())
//         .set_port(0)
//         .set_manager(manager)
//         .build()
//         .await;

//     let (tx_handler, mut rx_handler) =
//         tokio::sync::mpsc::channel::<(Message, SocketAddr)>(MAX_CONNECTIONS);

//     server.tx_handler = Some(tx_handler);

//     // let id = server.id;
//     // let start_time = server.start_time;
//     // while let Some(message) = rx_handler.recv().await {
//     //     // assert_eq!(message.0.action, Action::Join((id, start_time)));
//     //     return Ok(());
//     // };
//
//     let channel = tokio::task::spawn(async move {
//         println!("Spawned task");
//         if let Some((msg, _addr)) = rx_handler.recv().await {
//             println!("Received message: {:?}", msg);
//         };
//         println!("Finished task");
//     });

//     let (_,_) = tokio::join!(channel, server.listen());

//     Ok(())
// }
