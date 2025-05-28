use serde::{Deserialize, Serialize};
use native_db::*;
use native_model::{native_model, Model};
 
use once_cell::sync::Lazy;

#[derive(Serialize, Deserialize, Debug)]
#[native_model(id = 1, version = 1)]
#[native_db]
pub struct DatabaseEntry {
    #[primary_key]
    pub id: u64,
    #[secondary_key]
    pub record: (String, u32, u32),
}

pub static MODELS: Lazy<Models> = Lazy::new(|| {
   let mut models = Models::new();
   // It's a good practice to define the models by specifying the version
   models.define::<DatabaseEntry>().unwrap();
   models
});

pub fn add_record(db: &Database, name: String, wave: u32, time: u32) {
    let rw = db.rw_transaction().unwrap();
    let amount = rw.len().primary::<DatabaseEntry>().unwrap();
    rw.insert(DatabaseEntry {id: amount, record: (name, wave, time)}).unwrap();
    rw.commit().unwrap();
}

pub fn get_record(db: &Database, id: u64) -> Option<(String, u32, u32)> {
    let r = db.r_transaction().unwrap();
    let record: DatabaseEntry = r.get().primary(id).unwrap()?;

    Some(record.record)
}
