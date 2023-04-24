use std::sync::{Mutex, MutexGuard};

use once_cell::sync::Lazy;

static ID_MANAGER: Lazy<Mutex<IDManager>> = Lazy::new(|| {
    Mutex::new(IDManager {
        id_mappings: Vec::new(),
        next_id: Id(rand::random()),
    })
});

pub fn get_id_mgr() -> MutexGuard<'static, IDManager> {
    ID_MANAGER.lock().unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u64);

impl Id {
    pub const NULL: Id = Id(0);

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug)]
pub struct IDManager {
    id_mappings: Vec<Id>,
    next_id: Id,
}

impl IDManager {
    pub fn gen_id(&mut self) -> Id {
        self.next_id.0 += 1;
        Id(self.next_id.0 - 1)
    }

    pub fn gen_insert_zero(&mut self) -> Id {
        let id = self.gen_id();
        self.id_mappings.push(id);
        id
    }
}
