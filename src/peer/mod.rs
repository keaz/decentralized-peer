pub mod client;

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use async_std::net::TcpStream;
use uuid::Uuid;


pub struct Peer {
    pub peer_id: String,
    pub address: String,
    pub port: i32,
    pub stream: Arc<TcpStream>,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
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
        file_path: String
    },
    CreateFolder {
        id: Uuid,
        peer_id: String,
        folder_path: String
    },
    Test{
        id: Uuid,
        peer_id: String,
        message: String
    },
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum Event {
    Connected {
        id: Uuid,
        client_id: String,
        port: i32,
    },
    Left {
        id: Uuid,
        client_id: String,
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
#[serde(tag="peer_message")]
pub enum PeerMessage {
    PeerCommand{command: Command},
    PeerEvent{ event: Event},
}
