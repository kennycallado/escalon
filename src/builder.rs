use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use tokio::net::UdpSocket;

use crate::types::ClientState;
use crate::Escalon;

pub struct NoId;
pub struct Id(String);

pub struct NoAddr;
pub struct Addr(IpAddr);

pub struct NoPort;
pub struct Port(u16);

// pub struct NoCount;
// pub struct Count<J>(Arc<Mutex<Vec<J>>>);

pub struct EscalonBuilder<I, A, P> {
    pub id: I,
    pub addr: A,
    pub port: P,
}

impl EscalonBuilder<Id, Addr, Port> {
    pub async fn build<J>(self, count: Arc<Mutex<Vec<J>>>) -> Result<Escalon<J>> {
        let socket = UdpSocket::bind(format!("{:?}:{}", self.addr.0, self.port.0)).await?;
        socket.set_broadcast(true)?;

        let own_state = ClientState {
            memory: 0,
            tasks: 10,
        };

        let server = Escalon {
            id: self.id.0,
            clients: Arc::new(Mutex::new(HashMap::new())),
            count,
            // count: self.count.0,
            own_state: Arc::new(Mutex::new(own_state)),
            socket: Arc::new(socket),
            start_time: std::time::SystemTime::now(),
            tx_handler: None,
            tx_sender: None,
        };

        Ok(server)
    }
}

impl<I, A, P> EscalonBuilder<I, A, P> {
    pub fn set_id(self, id: impl Into<String>) -> EscalonBuilder<Id, A, P> {
        EscalonBuilder {
            id: Id(id.into()),
            addr: self.addr,
            port: self.port,
        }
    }
    pub fn set_addr(self, addr: IpAddr) -> EscalonBuilder<I, Addr, P> {
        EscalonBuilder {
            id: self.id,
            addr: Addr(addr),
            port: self.port,
        }
    }

    pub fn set_port(self, port: u16) -> EscalonBuilder<I, A, Port> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            port: Port(port),
        }
    }
}
