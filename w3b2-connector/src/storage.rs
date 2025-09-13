use anyhow::Result;
use sled::Db;

pub struct Storage {
    db: Db,
}

impl Storage {
    pub fn new(path: &str) -> Result<Self> {
        Ok(Self {
            db: sled::open(path)?,
        })
    }

    pub fn get_last_slot(&self) -> u64 {
        self.db
            .get("last_slot")
            .ok()
            .flatten()
            .and_then(|v| String::from_utf8(v.to_vec()).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    }

    pub fn set_last_slot(&self, slot: u64) {
        let _ = self.db.insert("last_slot", slot.to_string().as_bytes());
        let _ = self.db.flush();
    }
}
