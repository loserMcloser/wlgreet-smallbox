use smithay_client_toolkit::shm::MemPool;
use wayland_client::protocol::wl_shm;
use wayland_client::{Attached, Main};

pub struct DoubleMemPool {
    pool1: MemPool,
    pool2: MemPool,
    switch: bool,
}

impl DoubleMemPool {
    pub fn new(shm: Main<wl_shm::WlShm>) -> ::std::io::Result<DoubleMemPool> {
        Ok(DoubleMemPool {
            pool1: MemPool::new(Attached::from(shm.clone()), move |_| {})?,
            pool2: MemPool::new(Attached::from(shm), move |_| {})?,
            switch: false,
        })
    }

    pub fn pool(&mut self) -> Option<(&mut MemPool, &mut MemPool)> {
        let switch = self.switch;
        self.switch = !self.switch;
        let (last, cur) = if switch {
            (&mut self.pool2, &mut self.pool1)
        } else {
            (&mut self.pool1, &mut self.pool2)
        };

        if cur.is_used() {
            None
        } else {
            Some((last, cur))
        }
    }
}
