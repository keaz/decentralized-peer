pub mod client;

use async_std::net::TcpStream;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub struct Peer {
    pub peer_id: String,
    pub address: String,
    pub port: i32,
    pub stream: Arc<TcpStream>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Connect {
        id: Uuid,
        client_id: String,
        port: i32,
    },
    Leave {
        id: Uuid,
        client_id: String,
    },
    CreateNewFile {
        id: Uuid,
        peer_id: String,
        file_path: String,
        sha: String,
    },
    CreateFolder {
        id: Uuid,
        peer_id: String,
        folder_path: String,
    },
    ModifyFile {
        id: Uuid,
        peer_id: String,
        file_path: String,
    },
    DataRequestCommand {
        id: Uuid,
        peer_id: String,
        file_path: String,
    },
    WriteDataCommand {
        id: Uuid,
        peer_id: String,
        file_path: String,
        offset: u64,
        data: Vec<u8>,
    },
    Test {
        id: Uuid,
        peer_id: String,
        message: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    Connected {
        id: Uuid,
        client_id: String,
        port: i32,
    },
    Left {
        id: Uuid,
        client_id: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "peer_message")]
pub enum PeerMessage {
    PeerCommand { command: Command },
    PeerEvent { event: Event },
}
