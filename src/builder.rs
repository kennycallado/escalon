use std::net::IpAddr;
use std::sync::Arc;

use crate::{EscalonBuilder, EscalonTrait};

pub struct NoId;
pub struct Id(pub String);

pub struct NoAddr;
pub struct Addr(pub IpAddr);

pub struct NoPort;
pub struct Port(pub u16);

pub struct NoService;
pub struct Service(pub IpAddr);

pub struct NoManager;
pub struct Manager(pub Arc<dyn EscalonTrait>);

impl<I, A, S, P, F> EscalonBuilder<I, A, S, P, F> {
    pub fn set_id(self, id: impl Into<String>) -> EscalonBuilder<Id, A, S, P, F> {
        EscalonBuilder {
            id: Id(id.into()),
            addr: self.addr,
            svc: self.svc,
            port: self.port,
            manager: self.manager,
        }
    }

    pub fn set_addr(self, addr: IpAddr) -> EscalonBuilder<I, Addr, S, P, F> {
        EscalonBuilder {
            id: self.id,
            addr: Addr(addr),
            svc: self.svc,
            port: self.port,
            manager: self.manager,
        }
    }

    /// Set the service address for udp broadcast
    /// in kubernetes is the service name
    /// elsewhere "255.255.255.255"
    pub fn set_svc(self, svc: IpAddr) -> EscalonBuilder<I, A, Service, P, F> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            svc: Service(svc),
            port: self.port,
            manager: self.manager,
        }
    }

    pub fn set_port(self, port: u16) -> EscalonBuilder<I, A, S, Port, F> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            svc: self.svc,
            port: Port(port),
            manager: self.manager,
        }
    }

    pub fn set_manager(
        self,
        fucntions: impl EscalonTrait + Send + Sync + 'static,
    ) -> EscalonBuilder<I, A, S, P, Manager> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            svc: self.svc,
            port: self.port,
            manager: Manager(Arc::new(fucntions)),
        }
    }
}
