use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use tokio::net::UdpSocket;

use crate::server::{Callback, Server};

pub struct NoAddr;
pub struct Addr(IpAddr);

pub struct NoPort;
pub struct Port(u16);

pub struct NoCount;
pub struct Count(Callback);

pub struct ServerBuilder<A, P, C> {
    pub id: String,
    pub addr: A,
    pub port: P,
    pub count: C,
    // tx_up: Option<Sender<Message>>,
}

impl ServerBuilder<Addr, Port, Count> {
    pub async fn build(&self) -> Result<Server> {
        let socket = UdpSocket::bind(format!("{:?}:{}", self.addr.0, self.port.0)).await?;
        socket.set_broadcast(true)?;

        let server = Server {
            id: self.id.clone(),
            socket: Arc::new(socket),
            clients: Arc::new(Mutex::new(HashMap::new())),
            start_time: std::time::SystemTime::now(),
            tx_handler: None,
            tx_sender: None,
            count: self.count.0.clone(),
            // tx_up,
        };

        Ok(server)
    }
}

impl<A, P, C> ServerBuilder<A, P, C> {
    pub fn set_addr(self, addr: IpAddr) -> ServerBuilder<Addr, P, C> {
        ServerBuilder {
            id: self.id,
            addr: Addr(addr),
            port: self.port,
            count: self.count,
            // tx_up: self.tx_up,
        }
    }

    pub fn set_port(self, port: u16) -> ServerBuilder<A, Port, C> {
        ServerBuilder {
            id: self.id,
            addr: self.addr,
            port: Port(port),
            count: self.count,
            // tx_up: self.tx_up,
        }
    }

    // pub fn set_count(self, count: Callback) -> ServerBuilder<A, P, Count> {
    pub fn set_count(
        self,
        count: impl Fn() -> usize + Send + Sync + 'static,
    ) -> ServerBuilder<A, P, Count> {
        ServerBuilder {
            id: self.id,
            addr: self.addr,
            port: self.port,
            count: Count(Arc::new(count)),
            // tx_up: self.tx_up,
        }
    }

    // pub fn set_sender(mut self, tx_up: Sender<Message>) -> Self {
    //     self.tx_up = Some(tx_up);

    //     self
    // }
}
