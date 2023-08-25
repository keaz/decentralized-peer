use std::{sync::Arc, collections::{HashMap, hash_map::Entry}};

use async_std::{net::TcpStream, sync::Mutex, stream::StreamExt, task, io::WriteExt};
use futures::{select, FutureExt};
use log::{debug, info, warn};
use log4rs::append::file;
use uuid::Uuid;

use crate::{io::file_handler::FileHandler, Receiver, Result, peer::{Peer, PeerMessage, Command}};


//This is internal, within same process
pub enum InternalMessage {
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
    InternalToExternal { message: InternalToExternal},
    ExternalToInternal { message: ExternalToInternal},
}

pub enum InternalToExternal{
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
    RequestData {
        id: Uuid,
        file: String,
        peer_id: String,
        sha: String,
    }
}

pub enum ExternalToInternal{
    DataRequest {
        id: Uuid,
        peer_id: String,
        file_path: String,
    },
    DataWrite{
        id: Uuid,
        peer_id: String,
        file_path: String,
        offset: u64,
        data: Vec<u8>,
    },
    NewFileCreate {
        id: Uuid,
        peer_id: String,
        file_path: String,
        sha: String,
    },
}

pub struct Broker {
    my_peer_id: String,
    files_in_update: Arc<Mutex<HashMap<String,String>>>,
    file_handler: FileHandler,
}


impl Broker {
    pub fn new(
        my_peer_id: String, file_handler: FileHandler) -> Self {
            Broker {  my_peer_id, files_in_update: Arc::new(Mutex::new(HashMap::new())),file_handler  }
    }
}

impl Broker {
    //Handles only Message
    pub async fn broker_loop(&self, events: Receiver<InternalMessage>) -> Result<()> {
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
                InternalMessage::LeavePeer {
                    id,
                    peer_id: client_id,
                } => handle_peer_leave(&mut peers, client_id, &id),
                InternalMessage::NewPeer {
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
                InternalMessage::ExternalToInternal { message } => {
                    self.handle_external_to_internal(message, &mut peers).await.unwrap();
                },
                InternalMessage::InternalToExternal { message } => {
                    self.handle_internal_to_external(message, &mut peers).await.unwrap();
                }
            }    
        }
        drop(peers);
        // drop(disconnect_sender);
        // while let Some((_name, _pending_messages)) = disconnect_receiver.next().await {}
        Ok(())
    }
    
    pub async fn handle_internal_to_external(&self, message: InternalToExternal,  peers: &mut HashMap<String, Peer>) -> Result<()>{
        match message {
            InternalToExternal::FileCreated { id, file, sha } => {
                debug!(
                    "Recevied FileCreated {:?}  event id {:?}, sha {:?}",
                    file, id, sha
                );
                let files_in_update = self.files_in_update.clone();
                let files_in_update = files_in_update.lock().await;
                let is_updating = files_in_update.contains_key(&file);
                
                if is_updating {
                    return Ok(()) ;
                }
    
                peers.values().for_each(|peer| {
                    let command = PeerMessage::PeerCommand {
                        command: Command::CreateNewFile {
                            id,
                            file_path: file.clone(),
                            peer_id: self.my_peer_id.clone(),
                            sha: sha.clone(),
                        },
                    };
                    let command_json = serde_json::to_string(&command).unwrap();
                    task::block_on(send_message(peer.stream.clone(), command_json));
                });
    
                let files_in_update = self.files_in_update.clone();
    
                task::block_on(async {
                    let mut f = files_in_update.lock().await;
                    f.insert(file.clone(), sha.clone());
                });
            }
            InternalToExternal::FolderCreated { id, folder, sha } => {
                debug!(
                    "Recevied FolderCreated {:?}  event id {:?}, sha {:?} ",
                    folder, id, sha
                );
                peers.values().for_each(|peer| {
                    let command = PeerMessage::PeerCommand {
                        command: Command::CreateFolder {
                            id,
                            folder_path: folder.clone(),
                            peer_id: self.my_peer_id.clone(),
                        },
                    };
                    let command_json = serde_json::to_string(&command).unwrap();
                    task::block_on(send_message(peer.stream.clone(), command_json));
                });
            }
            InternalToExternal::FileModified { id, file, sha } => {
                debug!(
                    "Recevied FileModified {:?}  event id {:?}, sha {:?}",
                    file, id, sha
                );
                peers.values().for_each(|peer| {
                    let command = PeerMessage::PeerCommand {
                        command: Command::ModifyFile  {
                            id,
                            file_path: file.clone(),
                            peer_id: self.my_peer_id.clone(),
                        },
                    };
                    let command_json = serde_json::to_string(&command).unwrap();
                    task::block_on(send_message(peer.stream.clone(), command_json));
                });
            }
            InternalToExternal::FolderModified { id, folder, sha: _ } => {
                debug!("Recevied FolderModified {:?}  event id {:?} ", folder, id);
            }
            InternalToExternal::FileDeleted { id, folder, sha: _ } => {
                debug!("Recevied RemoveFolder {:?}  event id {:?} ", folder, id);
            }
            InternalToExternal::FolderDeleted { id, file, sha: _ } => {
                debug!("Recevied RemoveFile {:?}  event id {:?} ", file, id);
            },
            InternalToExternal::RequestData { id, file, peer_id, sha: _ } => {
                peers.values().filter(|peer| peer.peer_id.eq(&peer_id)).for_each(|peer| {
                    let command = PeerMessage::PeerCommand {
                        command: Command::DataRequestCommand {
                            id,
                            peer_id: self.my_peer_id.clone(),
                            file_path: file.clone(),
                        },
                    };
                    let command_json = serde_json::to_string(&command).unwrap();
                    task::block_on(send_message(peer.stream.clone(), command_json));
                });
            }
        }
        Ok(())
    }

    pub async fn handle_external_to_internal(&self, message: ExternalToInternal,  peers: &mut HashMap<String, Peer>) -> Result<()>{
        match message {
            ExternalToInternal::DataRequest { id, peer_id, file_path } => {
                let mut buf  = vec![0; 255];
                let mut offset = 0;
                while !self.file_handler.read_random(&file_path, offset, &mut buf).await.unwrap() {
                    
                    peers.values().filter(|peer| peer.peer_id.eq(&peer_id)).for_each(|peer| {
                        let command = PeerMessage::PeerCommand {
                            command: Command::WriteDataCommand { id, peer_id: self.my_peer_id.clone(), file_path: file_path.clone(), offset, data: buf.clone() },
                        };
                        let command_json = serde_json::to_string(&command).unwrap();
                        task::block_on(send_message(peer.stream.clone(), command_json));
                    });
                    offset = offset + 256;
                    buf.clear();
                }
            },
            ExternalToInternal::DataWrite { id: _, peer_id: _, file_path, offset, data } => {
                self.file_handler.write_random(file_path, offset, &data).await.unwrap();
            },
            ExternalToInternal::NewFileCreate { id, peer_id, file_path, sha } => {
                if self.file_handler.create_file(&file_path,&sha).await.unwrap() {
                    let files_in_update = self.files_in_update.clone();
                    let mut files_in_update = files_in_update.lock().await;
                    files_in_update.insert(file_path.clone(), sha);
                    peers.values().filter(|peer| peer.peer_id.eq(&peer_id)).for_each(|peer| {
                        let command = PeerMessage::PeerCommand {
                            command: Command::DataRequestCommand {
                                id,
                                peer_id: self.my_peer_id.clone(),
                                file_path: file_path.clone(),
                            },
                        };
                        let command_json = serde_json::to_string(&command).unwrap();
                        task::block_on(send_message(peer.stream.clone(), command_json));
                    });

                }
            },
        }
        Ok(())
    }

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

pub async fn send_message(stream: Arc<TcpStream>, peers_json: String) {
    let _rs = (&*stream).write_all(peers_json.as_bytes()).await;
    let _rs = (&*stream).write_all(b"\n").await;
}