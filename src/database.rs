use std::{sync::Arc, time::Duration};

use tokio::{fs::{self, File}, io::{AsyncBufReadExt, BufReader}, sync::Mutex};

pub struct Record {
    pub name: String,
    pub wave: u32,
    pub time: u32,
}

enum DatabaseParsingState {
    ParsingText,
    ParsingWave,
    ParsingTime,
}

pub struct Database {
    path: String,
    records: Vec<Record>,
}

impl Database {
    pub async fn new(path: String) -> Arc<Mutex<Database>> {
        let mut records = Vec::new();
        if !fs::try_exists(&path).await.expect("Invalid path") {
            fs::File::create_new(&path).await.unwrap();
        } else {
            let file = File::open(&path).await.unwrap();
            let reader = BufReader::new(file);
            let mut parsing_state = DatabaseParsingState::ParsingText;
            let mut current_record = Record {
                name: "".to_string(),
                wave: 0,
                time: 0,
            };

            let mut lines = reader.lines();

            loop {
                let line = lines.next_line().await.unwrap();
                if line.is_none() {
                    break;
                }
                let line = line.unwrap();

                match parsing_state {
                    DatabaseParsingState::ParsingText => {
                        current_record.name = line;
                        parsing_state = DatabaseParsingState::ParsingWave;
                    }
                    DatabaseParsingState::ParsingWave => {
                        current_record.wave = line.parse().unwrap();
                        parsing_state = DatabaseParsingState::ParsingTime;
                    }
                    DatabaseParsingState::ParsingTime => {
                        current_record.time = line.parse().unwrap();
                        records.push(current_record);
                        current_record = Record {
                            name: "".to_string(),
                            wave: 0,
                            time: 0,
                        };

                        parsing_state = DatabaseParsingState::ParsingText;
                    }
                }
            }
        }

        let db = Arc::new(Mutex::new(Database {
            path,
            records,
        }));
        let db_clone = db.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let db = db_clone.lock().await;

                let mut contents: Vec<u8> = Vec::new();
                for record in db.records.iter() {
                    contents.append(&mut record.name.as_bytes().to_vec());
                    contents.push(b'\n');
                    contents.append(&mut record.wave.to_string().as_bytes().to_vec());
                    contents.push(b'\n');
                    contents.append(&mut record.time.to_string().as_bytes().to_vec());
                    contents.push(b'\n');
                }

                fs::write(&db.path, &contents).await.expect("Failed writing to file");
            }
        });
        db
    }
}

pub fn add_record(db: &mut Database, name: String, mut wave: u32, mut time: u32) {
    let mut remove_idx: Option<usize> = None;
    for (i, other_record) in db.records.iter().enumerate() {
        if other_record.name == name {
            remove_idx = Some(i);
            
            if wave == other_record.wave {
                time = time.min(other_record.time);
            }
            wave = wave.max(other_record.wave);

            break;
        }
    }

    if let Some(idx) = remove_idx {
        db.records.remove(idx);
    }

    let record = Record {
        name,
        wave,
        time
    };

    if db.records.len() == 0 {
        db.records.push(record);
        return;
    }

    for (idx, other_record) in db.records.iter().enumerate() {
        if wave > other_record.wave || (wave == other_record.wave && time < other_record.time) {
            db.records.insert(idx, record);
            return;
        }
    }

    db.records.push(record);
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
