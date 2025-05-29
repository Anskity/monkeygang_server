use std::{sync::Arc, time::Duration};

use tokio::{fs, sync::Mutex};

pub struct Record {
    pub name: String,
    pub wave: u32,
    pub time: u32,
}

pub struct Database {
    path: String,
    records: Vec<Record>,
}

impl Database {
    pub async fn new(path: String) -> Arc<Mutex<Database>> {
        if !fs::try_exists(&path).await.expect("Invalid path") {
            fs::File::create_new(&path).await.unwrap();
        }

        let db = Arc::new(Mutex::new(Database {
            path,
            records: Vec::new(),
        }));
        let db_clone = db.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let db = db_clone.lock().await;

                let mut contents: Vec<u8> = Vec::new();
                for record in db.records.iter() {
                    contents.append(&mut record.name.as_bytes().to_vec());
                    contents.push(0);
                    contents.append(&mut record.wave.to_string().as_bytes().to_vec());
                    contents.push(0);
                    contents.append(&mut record.time.to_string().as_bytes().to_vec());
                    contents.push(0);
                }

                fs::write(&db.path, &contents).await.expect("Failed writing to file");
            }
        });
        db
    }
}

pub fn add_record(db: &mut Database, name: String, wave: u32, time: u32) {
    db.records.push(Record {
        name,
        wave,
        time
    });
}

pub fn get_records<'a>(db: &'a Database) -> Vec<&'a Record> {
    let mut vec: Vec<&'a Record> = Vec::new();

    for (i, record) in db.records.iter().enumerate() {
        if i >= 10 {
            break;
        }

        vec.push(record);
    }

    vec
}
