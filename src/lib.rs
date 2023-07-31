pub mod cmd;
pub mod core;
pub mod io;
pub mod peer;
pub mod rendezvous;
pub mod server;

use async_std::{net::TcpStream, prelude::*, task};
use futures::{channel::mpsc, select, FutureExt, SinkExt};
use log::{debug, info, warn};
use std::{
    collections::{hash_map::Entry, HashMap},
    future::Future,
    sync::Arc,
};
use uuid::Uuid;

use crate::io::file_handler::FileHandler;
use crate::peer::{Command, Event, Peer, PeerMessage};

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
        file: String,
        sha: String,
    },
    FolderCreated {
        id: Uuid,
        folder: String,
        sha: String,
    },
    FileModified {
        id: Uuid,
        file: String,
        sha: String,
    },
    FolderModified {
        id: Uuid,
        folder: String,
        sha: String,
    },
    FileDeleted {
        id: Uuid,
        folder: String,
        sha: String,
    },
    FolderDeleted {
        id: Uuid,
        file: String,
        sha: String,
    },
}

pub fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
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
            Message::LeavePeer {
                id,
                peer_id: client_id,
            } => handle_peer_leave(&mut peers, client_id, &id),
            Message::NewPeer {
                id: _,
                peer_id: client_id,
                address,
                port,
                stream,
            } => match peers.entry(client_id.clone()) {
                Entry::Occupied(..) => (),
                Entry::Vacant(entry) => {
                    entry.insert(Peer {
                        peer_id: client_id.clone(),
                        address,
                        port,
                        stream: stream.clone(),
                    });
                }
            },
            Message::FileCreated { id, file, sha } => {
                debug!(
                    "Recevied FileCreated {:?}  event id {:?}, sha {:?}",
                    file, id, sha
                );
                peers.values().for_each(|peer| {
                    let command = PeerMessage::PeerCommand {
                        command: Command::CreateNewFile {
                            id,
                            file_path: file.clone(),
                            peer_id: peer.peer_id.clone(),
                            sha: sha.clone(),
                        },
                    };
                    let command_json = serde_json::to_string(&command).unwrap();
                    task::block_on(send_message(peer.stream.clone(), command_json));
                });
            }
            Message::FolderCreated { id, folder, sha } => {
                debug!(
                    "Recevied FolderCreated {:?}  event id {:?}, sha {:?} ",
                    folder, id, sha
                );
                peers.values().for_each(|peer| {
                    let command = PeerMessage::PeerCommand {
                        command: Command::CreateFolder {
                            id,
                            folder_path: folder.clone(),
                            peer_id: peer.peer_id.clone(),
                        },
                    };
                    let command_json = serde_json::to_string(&command).unwrap();
                    task::block_on(send_message(peer.stream.clone(), command_json));
                });
            }
            Message::FileModified { id, file, sha } => {
                debug!("Recevied FileModified {:?}  event id {:?} ", file, id);
            }
            Message::FolderModified { id, folder, sha } => {
                debug!("Recevied FolderModified {:?}  event id {:?} ", folder, id);
            }
            Message::FileDeleted { id, folder, sha } => {
                debug!("Recevied RemoveFolder {:?}  event id {:?} ", folder, id);
            }
            Message::FolderDeleted { id, file, sha } => {
                debug!("Recevied RemoveFile {:?}  event id {:?} ", file, id);
            }
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
            warn!("Client already left id::{} client_id::{}", id, client_id);
        }
        Some(peer) => {
            info!("Client is leaving client_id::{}", peer.peer_id);
            drop(peer);
        }
    }
}

#[derive(Debug)]
pub struct PeerMessageHandler {
    file_handler: FileHandler,
}

impl PeerMessageHandler {
    
    pub fn new(file_handler: FileHandler) -> Self {
        PeerMessageHandler { file_handler }
    }

}

impl PeerMessageHandler {
    
    pub async fn handle_peer_message(&self, line: String, broker: &mut Sender<Message>) -> Result<()> {
        let message: PeerMessage = serde_json::from_str(&line)?;
        let _result = match message {
            PeerMessage::PeerCommand { command } => self.handle_command(command, broker).await,
            PeerMessage::PeerEvent { event } => self.handle_event(event, broker).await,
        };
    
        Ok(())
    }

    async fn handle_command(&self,command: Command, broker: &mut Sender<Message>) -> Result<()> {
        match command {
            Command::Connect {
                id,
                client_id,
                port,
            } => {
                //this should never happen, in this place
                warn!("Peer {} Connect command, id {} ", client_id, id);
            }
            Command::Leave { id, client_id } => {
                info!(
                    "Received Peer leave command id::{} client::{}",
                    id, client_id
                );
                broker
                    .send(Message::LeavePeer {
                        id,
                        peer_id: client_id,
                    })
                    .await
                    .unwrap();
            }
            Command::Test {
                id,
                peer_id,
                message,
            } => {
                info!(
                    "Peer {} test command with message {}, id :: {}",
                    peer_id, message, id
                );
            }
            Command::CreateNewFile {
                id,
                file_path,
                peer_id,
                sha,
            } => {
                info!(
                    "id :: {} Recevied CreateNewFile command for {} file from {}",
                    id, file_path, peer_id
                );
                self.file_handler.create_file(file_path,sha).await.unwrap();
            }
            Command::CreateFolder {
                id,
                folder_path,
                peer_id,
            } => {
                info!(
                    "id :: {} Recevied CreateFolder command for {} file from {}",
                    id, folder_path, peer_id
                );
            }
        }
        Ok(())
    }

    async fn handle_event(&self, event: Event, broker: &mut Sender<Message>) -> Result<()> {
        match event {
            Event::Connected {
                id,
                client_id,
                port,
            } => todo!(),
            Event::Left { id, client_id } => todo!(),
        }
    
        Ok(())
    }

}

pub fn get_available_port() -> Option<u16> {
    (8000..9000).find(|port| port_is_available(*port))
}

fn port_is_available(port: u16) -> bool {
    match std::net::TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
