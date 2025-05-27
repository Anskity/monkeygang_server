use std::{error::Error, ffi::CString, sync::Arc};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};
use tokio_postgres::types::ToSql;

struct Server {
    listener: TcpListener
}

type ClientPacketType = u8;
const PACKET_TYPE_GET_INFO: ClientPacketType = 0;
const PACKET_TYPE_SEND_RECORD: ClientPacketType = 1;

const PACKET_SIZE: usize = size_of::<ClientPacket>();

#[repr(C)]
#[derive(Clone, Copy)]
struct PacketGetInfo {
    
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PacketSendRecord {
    string_buffer: [u8; 128],
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
    name: [u8; 128],
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
    kind: ServerPacketType,
    data: ServerPacketData,
}

impl Server {
    pub async fn run(&mut self, postgres: Arc<tokio_postgres::Client>) {
        let (mut stream, addr) = self.listener.accept().await.expect("Error accepting connection");

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

            let message = unsafe {
                *(bytes.as_ptr() as *const ClientPacket)
            };

            match bytes[0] {
                PACKET_TYPE_GET_INFO => {
                    Server::handle_get_records(&mut stream, &unsafe{message.data.get_info}, postgres.clone()).await;
                }
                PACKET_TYPE_SEND_RECORD => {
                    Server::handle_send_record(&mut stream, &unsafe{message.data.send_record}, postgres.clone()).await;
                }
                _ => {
                    println!("Unknown message type: {}", bytes[0]);
                }
            }

            println!("Ending connection with: {}\n", addr);
        });
    }

    async fn handle_get_records(stream: &mut TcpStream, packet: &PacketGetInfo, postgres: Arc<tokio_postgres::Client>) {
        let query = r#"
            SELECT name, wave, time FROM Players
            ORDER BY wave DESC
            FETCH FIRST 10 ROWS ONLY
        "#;
        let rows = postgres.query(query, &[]).await.expect("Failed to read from database");
        assert!(rows.len() <= 10);

        let mut records = [Record {name: [0; 128], wave: 0, time: 0}; 10];

        for (i, row) in rows.iter().enumerate() {
            let name: String = row.get("name");
            let wave: u32 = row.get("wave");
            let time: u32 = row.get("time");

            let name = name.as_bytes();

            for chr_idx in 0..(name.len()) {
                records[i].name[chr_idx] = name[chr_idx];
            }
            records[i].wave = wave;
            records[i].time = time;
        }

        let packet: *const u8 = (&ServerPacketData {
            records,
        }) as *const ServerPacketData as *const u8;
        let slice = unsafe {std::slice::from_raw_parts(packet, size_of::<ServerPacketData>())};
        stream.write(slice).await.unwrap();
    }
    async fn handle_send_record(stream: &mut TcpStream, packet: &PacketSendRecord, postgres: Arc<tokio_postgres::Client>) {
        let query = r#"
            INSERT INTO Players (name, wave, time)
            VALUES ($1, $2, $3)
        "#;

        let name = match unsafe{CString::from_raw(packet.string_buffer.clone().as_ptr().cast_mut() as *mut i8)}.to_str() {
            Ok(value) => value,
            Err(err) => {
                println!("{:?}", err);
                return;
            }
        }.to_string();

        postgres.query(query, &[&name, &packet.wave, &packet.time]).await.unwrap();
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(format!("127.0.0.1:8080")).await?;
    let mut server = Server {
        listener
    };

    let (postgres_client, postgres_conn) = tokio_postgres::connect("host=localhost user=postgres", tokio_postgres::NoTls).await.unwrap();
    tokio::spawn(async move {
        if let Err(e) = postgres_conn.await {
            eprintln!("connection error: {}", e);
        }
    });

    let postgres = Arc::new(postgres_client);
    server.run(postgres).await;

    Ok(())
}
