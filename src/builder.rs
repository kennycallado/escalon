use std::net::IpAddr;
use std::sync::Arc;

use crate::{EscalonBuilder, FnAddJobs};

pub struct NoId;
pub struct Id(pub String);

pub struct NoAddr;
pub struct Addr(pub IpAddr);

pub struct NoPort;
pub struct Port(pub u16);

pub struct NoCount;
pub struct Count(pub Arc<dyn Fn() -> usize + Send + Sync>);

pub struct NoAddJobs;
pub struct AddJobs(pub Arc<FnAddJobs>);

impl<I, A, P, C, J> EscalonBuilder<I, A, P, C, J> {
    pub fn set_id(self, id: impl Into<String>) -> EscalonBuilder<Id, A, P, C, J> {
        EscalonBuilder {
            id: Id(id.into()),
            addr: self.addr,
            port: self.port,
            count: self.count,
            add_jobs: self.add_jobs,
        }
    }

    pub fn set_addr(self, addr: IpAddr) -> EscalonBuilder<I, Addr, P, C, J> {
        EscalonBuilder {
            id: self.id,
            addr: Addr(addr),
            port: self.port,
            count: self.count,
            add_jobs: self.add_jobs,
        }
    }

    pub fn set_port(self, port: u16) -> EscalonBuilder<I, A, Port, C, J> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            port: Port(port),
            count: self.count,
            add_jobs: self.add_jobs,
        }
    }

    pub fn set_count_jobs(
        self,
        count: impl Fn() -> usize + Send + Sync + 'static,
    ) -> EscalonBuilder<I, A, P, Count, J> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            port: self.port,
            count: Count(Arc::new(count)),
            add_jobs: self.add_jobs,
        }
    }

    pub fn set_take_jobs(
        self,
        add_jobs: impl Fn(&str, usize, usize) + Send + Sync + 'static,
    ) -> EscalonBuilder<I, A, P, C, AddJobs> {
        EscalonBuilder {
            id: self.id,
            addr: self.addr,
            port: self.port,
            count: self.count,
            add_jobs: AddJobs(Arc::new(add_jobs)),
        }
    }
}
