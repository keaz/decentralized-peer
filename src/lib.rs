pub mod cmd;
pub mod core;
pub mod io;
pub mod peer;
pub mod rendezvous;
pub mod server;
pub mod broker;

use async_std::{prelude::*, task};
use broker::{InternalMessage, ExternalToInternal};
use futures::{channel::mpsc, SinkExt};
use log::{debug, info, warn};
use std::{
    future::Future,
};



use crate::peer::{Command, Event, PeerMessage};

pub type Sender<T> = mpsc::UnboundedSender<T>;
pub type Receiver<T> = mpsc::UnboundedReceiver<T>;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;



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


#[derive(Debug)]
pub struct PeerMessageHandler {
}

impl PeerMessageHandler {
    
    pub fn new() -> Self {
        PeerMessageHandler {}
    }

}

impl PeerMessageHandler {
    
    pub async fn handle_peer_message(&self, line: String, broker: &mut Sender<InternalMessage>) -> Result<()> {
        let message: PeerMessage = serde_json::from_str(&line)?;
        let _result = match message {
            PeerMessage::PeerCommand { command } => self.handle_command(command, broker).await,
            PeerMessage::PeerEvent { event } => self.handle_event(event, broker).await,
        };
    
        Ok(())
    }

    async fn handle_command(&self,command: Command, broker: &mut Sender<InternalMessage>) -> Result<()> {
        match command {
            Command::Connect {
                id,
                client_id,
                port: _,
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
                    .send(InternalMessage::LeavePeer {
                        id,
                        peer_id: client_id,
                    })
                    .await
                    .unwrap();
            },
            Command::ModifyFile { id: _, peer_id: _, file_path: _ } => {
                debug!("Not Implemented yet");
            },
            Command::DataRequestCommand { id, peer_id, file_path } => {
                let message = ExternalToInternal::DataRequest { id, peer_id, file_path };
                broker.send(InternalMessage::ExternalToInternal { message }).await.unwrap();
            },
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
                let message = ExternalToInternal::NewFileCreate { id, peer_id, file_path, sha };
                broker.send(InternalMessage::ExternalToInternal { message }).await.unwrap();
                
                
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
            Command::WriteDataCommand { id, peer_id, file_path, offset, data } => {
                debug!(
                    "id :: {} Recevied Write Data command for {} file from {}",
                    id, file_path, peer_id
                );
                let message = ExternalToInternal::DataWrite { id, peer_id, file_path, offset, data };
                broker.send(InternalMessage::ExternalToInternal { message }).await.unwrap();
            },
        }
        Ok(())
    }

    async fn handle_event(&self, event: Event, _broker: &mut Sender<InternalMessage>) -> Result<()> {
        match event {
            Event::Connected {
                id: _,
                client_id: _,
                port: _,
            } => todo!(),
            Event::Left { id: _, client_id: _ } => todo!(),
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
