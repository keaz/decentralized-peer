use std::sync::Arc;

use async_std::{
    io::{stdin, BufReader},
    net::{TcpStream, ToSocketAddrs},
    prelude::*,
};

use futures::{select, FutureExt, SinkExt};

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{peer::client::ClientConnectionHandler, InternalMessage, Result, Sender};

#[derive(Serialize, Deserialize)]
pub enum ClientCommand {
    ConnectClient {
        id: String,
        client_id: String,
        port: i32,
    },
    LeaveClient {
        id: String,
        client_id: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientEvent {
    ClientConnected {
        id: Uuid,
        client_id: String,
        peers: Vec<ConnectedPeer>,
    },
    ClientLeft {
        id: Uuid,
        client_id: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectedPeer {
    pub peer_id: String,
    pub address: String,
    pub port: i32,
}

pub struct Server {
    client: Arc<ClientConnectionHandler>
}

impl Server {
    
    pub fn new(client: ClientConnectionHandler) -> Self {
        Server { client: Arc::new(client) }
    }

}

impl Server {
    pub async fn server_connection_loop(
        &self,
        addr: impl ToSocketAddrs,
        client_id: &String,
        mut broker_sender: Sender<InternalMessage>,
        available_port: i32,
    ) -> Result<()> {
        let stream = TcpStream::connect(addr).await?;
        let join_client_command = ClientCommand::ConnectClient {
            id: Uuid::new_v4().to_string(),
            client_id: client_id.clone(),
            port: available_port,
        };
        let event_json = serde_json::to_string(&join_client_command)?;
    
        let (reader, mut writer) = (&stream, &stream);
        send_event(event_json, &mut writer).await?;
        
        let mut lines_from_server = BufReader::new(reader).lines().fuse();
        let mut lines_from_stdin = BufReader::new(stdin()).lines().fuse();

        // let client = self.client.clone();
        loop {
            select! {
                line = lines_from_server.next().fuse() => match line {
                    Some(line) => {
                        let line = line?;
                        debug!("{}", line);
                        match serde_json::from_str(&line) {
                            Err(..) => {
                                warn!("Error converting JSON to ClientEvent");
                            }
                            Ok(server_event) => {
                                match server_event {
                                    ClientEvent::ClientConnected{ id: _,client_id:_,peers} => {
                                        
                                        for peer in peers {
                                            let client = self.client.clone();
                                            info!("Received peer: {}",peer.peer_id);
                                            let peer_address = format!("{}:{}",peer.address,peer.port);
                                            client.clone().client_connection(peer_address, broker_sender.clone()).await.unwrap();
                                        }
                                    },
                                    ClientEvent::ClientLeft{id,client_id} => {
                                        info!("Received peer leave event for peer: {}",client_id);
                                        let peer_leave_message = InternalMessage::LeavePeer{id: id.clone(),peer_id:client_id};
                                        broker_sender.send(peer_leave_message).await.unwrap();
                                    },
                                }
                            },
                        };
                    },
                    None => break,
                },
                line = lines_from_stdin.next().fuse() => match line {
                    Some(line) => {
                        let line = line?;
                        if line.eq("EXIT") {
                            send_exit_event(client_id.clone(), stream.clone()).await?;
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
        Ok(())
    }
}



async fn send_exit_event(client_id: String, stream: TcpStream) -> Result<()> {
    let id = Uuid::new_v4();
    let leave_command = ClientCommand::LeaveClient {
        id: id.to_string(),
        client_id: client_id.clone(),
    };
    let event_json = serde_json::to_string(&leave_command)?;

    send_event(event_json, &mut &stream).await?;
    info!("Sent exit event id::{}", id);
    Ok(())
}

async fn send_event(event_json: String, writer: &mut &TcpStream) -> Result<()> {
    writer.write_all(event_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}
