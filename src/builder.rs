use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use tokio::net::UdpSocket;

use crate::Escalon;

pub struct NoId;
pub struct Id(String);

pub struct NoAddr;
pub struct Addr(IpAddr);

pub struct NoPort;
pub struct Port(u16);

pub struct NoCount;
pub struct Count(Arc<dyn Fn() -> usize + Send + Sync>);

pub struct EscalonBuilder<I, A, P, C> {
    pub id: I,
    pub addr: A,
    pub port: P,
    pub count: C,
    // tx_up: Option<Sender<Message>>,
}

impl EscalonBuilder<Id, Addr, Port, Count> {
    pub async fn build(self) -> Result<Escalon> {
        let socket = UdpSocket::bind(format!("{:?}:{}", self.addr.0, self.port.0)).await?;
        socket.set_broadcast(true)?;

        let server = Escalon {
            id: self.id.0,
            socket: Arc::new(socket),
            clients: Arc::new(Mutex::new(HashMap::new())),
            start_time: std::time::SystemTime::now(),
            tx_handler: None,
            tx_sender: None,
            count: self.count.0,
            // tx_up,
        };

        Ok(server)
    }
}

impl<I, A, P, C> EscalonBuilder<I, A, P, C> {
    pub fn set_id(self, id: impl Into<String>) -> EscalonBuilder<Id, A, P, C> {
        EscalonBuilder {
            id: Id(id.into()),
            addr: self.addr,
            port: self.port,
            count: self.count,
            // tx_up: self.tx_up,
        }
    }
    pub fn set_addr(self, addr: IpAddr) -> EscalonBuilder<I,Addr, P, C> {
        EscalonBuilder {
            id: self.id,
            addr: Addr(addr),
            port: self.port,
            count: self.count,
            // tx_up: self.tx_up,
        }
    }

    pub fn set_port(self, port: u16) -> EscalonBuilder<I, A, Port, C> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            port: Port(port),
            count: self.count,
            // tx_up: self.tx_up,
        }
    }

    pub fn set_count(
        self,
        count: impl Fn() -> usize + Send + Sync + 'static,
    ) -> EscalonBuilder<I,A, P, Count> {
        EscalonBuilder {
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
