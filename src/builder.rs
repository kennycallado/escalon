use std::net::IpAddr;
use std::sync::Arc;

use crate::{EscalonBuilder, EscalonTrait};

pub struct NoId;
pub struct Id(pub String);

pub struct NoAddr;
pub struct Addr(pub IpAddr);

pub struct NoPort;
pub struct Port(pub u16);

pub struct NoManager;
pub struct Manager(pub Arc<dyn EscalonTrait>);

impl<I, A, P, F> EscalonBuilder<I, A, P, F> {
    pub fn set_id(self, id: impl Into<String>) -> EscalonBuilder<Id, A, P, F> {
        EscalonBuilder {
            id: Id(id.into()),
            addr: self.addr,
            port: self.port,
            manager: self.manager,
        }
    }

    pub fn set_addr(self, addr: IpAddr) -> EscalonBuilder<I, Addr, P, F> {
        EscalonBuilder {
            id: self.id,
            addr: Addr(addr),
            port: self.port,
            manager: self.manager,
        }
    }

    pub fn set_port(self, port: u16) -> EscalonBuilder<I, A, Port, F> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            port: Port(port),
            manager: self.manager,
        }
    }

    pub fn set_manager(
        self,
        fucntions: impl EscalonTrait + Send + Sync + 'static,
    ) -> EscalonBuilder<I, A, P, Manager> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            port: self.port,
            manager: Manager(Arc::new(fucntions)),
        }
    }
}
