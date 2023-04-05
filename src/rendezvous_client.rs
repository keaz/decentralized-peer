use async_std::{
    io::{BufReader, stdin},
    net::{TcpStream, ToSocketAddrs},
    prelude::*,
};

use futures::{FutureExt, select, SinkExt};

use log::{debug, info, warn};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::{Sender,Result, Message, peer_client::client_connection, spawn_and_log_error};

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
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientEvent {
    ClientConnected {
        id: Uuid,
        client_id: String,
        peers: Vec<ConnectedPeer>,
    },
    ClientLeft{
        id: Uuid,
        client_id: String,
    }
}


#[derive(Serialize, Deserialize,Debug)]
pub struct ConnectedPeer{
    pub peer_id: String,
    pub address: String,
    pub port: i32,
}

pub async fn server_connection_loop(addr: impl ToSocketAddrs, client_id: &String, mut broker_sender: Sender<Message>) -> Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let join_client_command = ClientCommand::ConnectClient { id: Uuid::new_v4().to_string(), client_id: client_id.clone(), port: 7890 };
    let event_json = serde_json::to_string(&join_client_command)?;

    let (reader, mut writer) = (&stream, &stream); 
    send_event(event_json, &mut writer).await?;

    let mut lines_from_server = BufReader::new(reader).lines().fuse();
    let mut lines_from_stdin = BufReader::new(stdin()).lines().fuse();
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
                                        info!("Received peer: {}",peer.peer_id);
                                        let peer_address = format!("{}:{}",peer.address,peer.port);
                                        spawn_and_log_error(client_connection(peer_address, broker_sender.clone()));
                                    }
                                },
                                ClientEvent::ClientLeft{id,client_id} => {
                                    info!("Received peer leave event for peer: {}",client_id);
                                    let peer_leave_message = Message::LeavePeer{id: id.clone(),peer_id:client_id};
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


async fn send_exit_event(client_id: String,stream:TcpStream ) -> Result<()> {
    let id = Uuid::new_v4();
    let leave_command = ClientCommand::LeaveClient{id: id.to_string(), client_id: client_id.clone()};
    let event_json = serde_json::to_string(&leave_command)?;

    send_event(event_json, &mut &stream).await?;
    info!("Sent exit event id::{}",id);
    Ok(())
}

async fn send_event(event_json: String, writer: &mut &TcpStream) -> Result<()> {
    writer.write_all(event_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}


