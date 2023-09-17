use anyhow::Result;

use super::*;

use crate::constants::MAX_CONNECTIONS;
use crate::types::message::Action;

#[tokio::test]
async fn test_server_creation_and_listen() -> Result<()> {
    let blah = vec![1, 2, 3];
    let blah = Arc::new(blah);

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_port(0) // Use a random available port
        .set_count(move || { blah.len() })
        .build()
        .await?;

    assert!(server.listen().await.is_ok());

    drop(server);

    Ok(())
}

#[tokio::test]
#[should_panic]
async fn test_bind_twice() {
    let blah = vec![1, 2, 3];
    let blah = Arc::new(blah);

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_port(0) // Use a random available port
        .set_count(move || { blah.len() })
        .build()
        .await
        .unwrap();

    assert!(server.listen().await.is_ok());
    tokio::net::UdpSocket::bind(server.socket.local_addr().unwrap()).await.unwrap();

    drop(server);
}

#[tokio::test]
#[should_panic]
async fn test_server_invalid_port() {
    let blah = vec![1, 2, 3];
    let blah = Arc::new(blah);

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_port(1)
        .set_count(move || { blah.len() })
        .build()
        .await
        .unwrap();

    assert!(server.listen().await.is_ok());

    drop(server);
}

#[tokio::test]
async fn test_intercept_before_send_join() -> Result<()> {
    let blah = vec![1, 2, 3];
    let blah = Arc::new(blah);

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_port(0) // Use a random available port
        .set_count(move || { blah.len() })
        .build()
        .await?;

    let (tx_sender, mut rx_sender) =
        tokio::sync::mpsc::channel::<(Message, Option<SocketAddr>)>(MAX_CONNECTIONS);
    server.tx_sender = Some(tx_sender);

    assert!(server.send_join().is_ok());

    let received_message: (Message, Option<SocketAddr>) =
        rx_sender.recv().await.unwrap();

    let id = server.id;
    let start_time = server.start_time;
    assert_eq!(received_message.1, None);
    assert_eq!(received_message.0.action, Action::Join((id, start_time)));

    Ok(())
}

#[tokio::test]
async fn test_intercept_before_hertbeat() -> Result<()> {
    let blah = vec![1, 2, 3];
    let blah = Arc::new(blah);

    let mut server = Escalon::new()
        .set_id("test")
        .set_addr("127.0.0.1".parse().unwrap())
        .set_port(0)
        .set_count(move || { blah.len() })
        .build()
        .await?;

    let (tx_sender, mut rx_sender) =
        tokio::sync::mpsc::channel::<(Message, Option<SocketAddr>)>(MAX_CONNECTIONS);
    server.tx_sender = Some(tx_sender);

    assert!(server.start_heartbeat().is_ok());

    let received_message: (Message, Option<SocketAddr>) =
        rx_sender.recv().await.unwrap();

    assert_eq!(received_message.1, None);
    assert!(matches!(received_message.0.action, Action::Check(..)));

    Ok(())
}

// IT DOESN'T WORK
// #[tokio::test]
// #[should_panic]
// async fn test_intercept_after_send_join() {
//     let blah = vec![1, 2, 3];
//     let blah = Arc::new(blah);

//     let mut server = Escalon::new()
//         .set_id("test")
//         .set_addr("127.0.0.1".parse().unwrap())
//         .set_port(0)
//         .set_count(move || { blah.len() })
//         .build()
//         .await
//         .unwrap();

//     let (tx_handler, mut rx_handler) =
//         tokio::sync::mpsc::channel::<(Message, SocketAddr)>(MAX_CONNECTIONS);

//     server.tx_handler = Some(tx_handler);

//     server.listen().await.unwrap();

//     // let id = server.id;
//     // let start_time = server.start_time;
//     // while let Some(message) = rx_handler.recv().await {
//     //     // assert_eq!(message.0.action, Action::Join((id, start_time)));
//     //     return Ok(());
//     // };

//     if let None = rx_handler.recv().await {
//         panic!("blah");
//     };
// }
