pub mod core;
pub mod peer_server;
pub mod peer_client;
pub mod peer;
pub mod file_check;


use std::{future::Future, sync::Arc, collections::{HashMap, hash_map::Entry}};
use futures::{channel::mpsc, select, FutureExt, SinkExt};
use async_std::{task, net::TcpStream, prelude::*,};
use log::{warn, info};
use uuid::Uuid;

use crate::peer::{Peer, PeerMessage, Command, Event};

pub type Sender<T> = mpsc::UnboundedSender<T>;
pub type Receiver<T> = mpsc::UnboundedReceiver<T>;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

//This is internal, within same process
pub enum Message {
    NewPeer {
        id: Uuid,
        peer_id: String,
        address: String,
        port: i32,
        stream: Arc<TcpStream>,
    },
    LeavePeer {
        id: Uuid,
        peer_id: String,
    },
    FileCreated {
        id: Uuid,
        file: String
    },
    FolderCreated {
        id: Uuid,
        folder: String
    },

}

pub fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()> where F: Future<Output = Result<()>> + Send + 'static,{
    task::spawn(async move {
        if let Err(e) = fut.await {
            warn!("{}", e)
        }
    })
}

pub async fn send_message(stream: Arc<TcpStream>, peers_json: String) {
    let _rs = (&*stream).write_all(peers_json.as_bytes()).await;
    let _rs = (&*stream).write_all(b"\n").await;
}

//Handles only Message
pub async fn broker_loop(events: Receiver<Message>) -> Result<()> {
    // let (disconnect_sender, mut disconnect_receiver) = mpsc::unbounded::<(String, Receiver<PeerEvent>)>();
    let mut peers: HashMap<String, Peer> = HashMap::new();
    let mut events = events.fuse();
    loop {
        let event = select! {
            event = events.next().fuse() => match event {
                None => break, // 2
                Some(event) => event,
            },
        };
        match event {
            Message::LeavePeer { id, peer_id: client_id } => handle_peer_leave(&mut peers, client_id, &id),
            Message::NewPeer {id: _,peer_id: client_id, address,port, stream,} => match peers.entry(client_id.clone()) {
                Entry::Occupied(..) => (),
                Entry::Vacant(entry) => {
                    entry.insert(Peer {peer_id: client_id.clone(),address,port, stream: stream.clone(),});
                }
            },
            Message::FileCreated { id, file } => {
                info!("Recevied File {:?}  event id {:?} ",file, id);
                peers.values().for_each(|peer| {
                    let command = PeerMessage::PeerCommand { command: Command::CreateNewFile { id, file_path: file.clone(), peer_id: peer.peer_id.clone() } };
                    let command_json = serde_json::to_string(&command).unwrap();
                    task::block_on(send_message(peer.stream.clone(), command_json));
                });
                
            },
            Message::FolderCreated { id, folder } => {
                peers.values().for_each(|peer| {
                    let command = PeerMessage::PeerCommand { command: Command::CreateFolder { id, folder_path: folder.clone(), peer_id: peer.peer_id.clone() } };
                    let command_json = serde_json::to_string(&command).unwrap();
                    task::block_on(send_message(peer.stream.clone(), command_json));
                });
            },
        }
    }
    drop(peers); 
    // drop(disconnect_sender);
    // while let Some((_name, _pending_messages)) = disconnect_receiver.next().await {}
    Ok(())
}

fn handle_peer_leave(peers: &mut HashMap<String, Peer>, client_id: String, id: &Uuid) {
    match peers.remove(&client_id) {
        None => {
            warn!("Client already left id::{} client_id::{}", id, client_id)
        }
        Some(peer) => {
            info!("Client is leaving client_id::{}", peer.peer_id);
            drop(peer);
        }
    }
}


pub async fn handle_peer_message(line: String, broker: &mut Sender<Message>) -> Result<()> {

    let message: PeerMessage = serde_json::from_str(&line)?;
    let _result  = match message {
        PeerMessage::PeerCommand { command } => handle_command(command, broker).await,
        PeerMessage::PeerEvent { event } => handle_event(event, broker).await,
    };

    Ok(())
}

async fn handle_command(command: Command,  broker: &mut Sender<Message>) -> Result<()> {
    match command {
        Command::Connect { id, client_id, port } => {
            //this should never happen, in this place
            warn!("Peer {} Connect command, id {} ",client_id,id);
        },
        Command::Leave { id, client_id } => {
            info!("Received Peer leave command id::{} client::{}",id, client_id);
                broker
                    .send(Message::LeavePeer { id, peer_id: client_id })
                    .await
                    .unwrap();
        },
        Command::Test { id, peer_id, message } => {
            info!("Peer {} test command with message {}, id :: {}",peer_id,message,id);
        },
        Command::CreateNewFile { id, file_path, peer_id } => {
            info!("id :: {} Recevied CreateNewFile command for {} file from {}",id,file_path,peer_id);
        },
        Command::CreateFolder { id, folder_path, peer_id } => {
            info!("id :: {} Recevied CreateFolder command for {} file from {}",id,folder_path,peer_id);
        },
    }
    Ok(())
}

async fn handle_event(event: Event,  broker: &mut Sender<Message>) -> Result<()> {

    match event {
        Event::Connected { id, client_id, port } => todo!(),
        Event::Left { id, client_id } => todo!(),
    }

    Ok(())
}