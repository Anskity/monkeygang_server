use std::{error::Error, sync::Arc};

use monkeygang_server::{
    STRING_BUFFER_SIZE,
    database::{self, Database, get_records},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

struct Server {
    listener: TcpListener,
}

type ClientPacketType = u8;
const PACKET_TYPE_GET_INFO: ClientPacketType = 0;
const PACKET_TYPE_SEND_RECORD: ClientPacketType = 1;

const PACKET_SIZE: usize = size_of::<ClientPacket>();

#[repr(C)]
#[derive(Clone, Copy)]
struct PacketGetInfo {}

#[repr(C)]
#[derive(Clone, Copy)]
struct PacketSendRecord {
    string_buffer: [u8; STRING_BUFFER_SIZE],
    wave: u32,
    time: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
union ClientPacketData {
    get_info: PacketGetInfo,
    send_record: PacketSendRecord,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ClientPacket {
    kind: ClientPacketType,
    data: ClientPacketData,
}

type ServerPacketType = u8;
const SERVER_PACKET_TYPE_RECORDS: ServerPacketType = 0;

#[derive(Clone, Copy)]
#[repr(C)]
struct Record {
    name: [u8; STRING_BUFFER_SIZE],
    wave: u32,
    time: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
union ServerPacketData {
    records: [Record; 10],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ServerPacket {
    pub kind: ServerPacketType,
    pub data: ServerPacketData,
}

impl Server {
    pub async fn run(&mut self, db: Arc<Mutex<Database>>) {
        let (mut stream, addr) = self
            .listener
            .accept()
            .await
            .expect("Error accepting connection");

        tokio::spawn(async move {
            let addr = addr.to_string();
            println!("Starting connection with: {}", addr);

            let mut bytes: [u8; PACKET_SIZE] = [0; PACKET_SIZE];
            let n: usize = match stream.read(&mut bytes).await {
                Ok(n) => n,
                Err(err) => {
                    println!("Got an error: {}", err);
                    return;
                }
            };

            if n == 0 {
                println!("Got an empty byte stream");
            }

            let message = unsafe { *(bytes.as_ptr() as *const ClientPacket) };

            match bytes[0] {
                PACKET_TYPE_GET_INFO => {
                    Server::handle_get_records(
                        &mut stream,
                        &unsafe { message.data.get_info },
                        db.clone(),
                    )
                    .await;
                }
                PACKET_TYPE_SEND_RECORD => {
                    Server::handle_send_record(
                        &mut stream,
                        &unsafe { message.data.send_record },
                        db.clone(),
                    )
                    .await;
                }
                _ => {
                    println!("Unknown message type: {}", bytes[0]);
                }
            }

            println!("Ending connection with: {}\n", addr);
        });
    }

    async fn handle_get_records(
        stream: &mut TcpStream,
        _: &PacketGetInfo,
        db: Arc<Mutex<Database>>,
    ) {
        //
        // let len = times.len();
        // let idxs: Vec<usize> = (0..len).collect();
        // quick_sort(&mut idxs);

        let mut records = [Record {
            name: [0; STRING_BUFFER_SIZE],
            wave: 0,
            time: 0,
        }; 10];

        let mut db_value = db.lock().await;
        let records_vec = get_records(&mut db_value);
        for (i, record) in records_vec.iter().enumerate() {
            let str_bytes = record.name.as_bytes();

            for (j, byte) in str_bytes.iter().enumerate() {
                records[i].name[j] = *byte;
            }
            records[i].wave = record.wave;
            records[i].time = record.time;
        }

        let packet = (&ServerPacket {
            kind: SERVER_PACKET_TYPE_RECORDS,
            data: ServerPacketData { records },
        }) as *const ServerPacket as *const u8;
        let slice = unsafe { std::slice::from_raw_parts(packet, size_of::<ServerPacket>()) };
        stream.write(slice).await.unwrap();
    }
    async fn handle_send_record(
        _: &mut TcpStream,
        packet: &PacketSendRecord,
        db: Arc<Mutex<Database>>,
    ) {
        let name = String::from_utf8(packet.string_buffer.to_vec());

        if let Err(err) = name {
            dbg!(err);
            return;
        }
        let name = name.unwrap();

        if !name.is_ascii() {
            return;
        }

        let mut db = db.lock().await;
        database::add_record(&mut db, name, packet.wave, packet.time);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(format!("127.0.0.1:8080")).await?;
    let mut server = Server { listener };

    let db = Database::new("db.txt".to_string()).await;
    server.run(db).await;

    Ok(())
}
