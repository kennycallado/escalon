use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use tokio::net::UdpSocket;

use crate::server::Server;

pub struct NoAddr;
pub struct Addr(IpAddr);

pub struct NoPort;
pub struct Port(u16);

pub struct ServerBuilder<A, P> {
    id: String,
    addr: A,
    port: P,
    // tx_up: Option<Sender<Message>>,
}

impl ServerBuilder<NoAddr, NoPort> {
    pub fn new(id: String) -> Self {
        Self {
            id,
            addr: NoAddr,
            port: NoPort,
            // tx_up: None,
        }
    }
}

impl ServerBuilder<Addr, Port> {
    pub async fn build(self) -> Result<Server> {
        let socket = UdpSocket::bind(format!("{:?}:{}", self.addr.0, self.port.0)).await?;
        socket.set_broadcast(true)?;

        let server = Server {
            id: self.id,
            socket: Arc::new(socket),
            clients: Arc::new(Mutex::new(HashMap::new())),
            tx_handler: None,
            tx_sender: None,
            // tx_up,
        };

        Ok(server)
    }
}

impl<A, P> ServerBuilder<A, P> {
    pub fn set_addr(self, addr: IpAddr) -> ServerBuilder<Addr, P> {
        ServerBuilder {
            id: self.id,
            addr: Addr(addr),
            port: self.port,
            // tx_up: self.tx_up,
        }
    }

    pub fn set_port(self, port: u16) -> ServerBuilder<A, Port> {
        ServerBuilder {
            id: self.id,
            addr: self.addr,
            port: Port(port),
            // tx_up: self.tx_up,
        }
    }

    // pub fn set_sender(mut self, tx_up: Sender<Message>) -> Self {
    //     self.tx_up = Some(tx_up);

    //     self
    // }
}
