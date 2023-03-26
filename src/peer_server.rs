use std::sync::Arc;

use async_std::{
    io::BufReader,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    prelude::*,
};
use futures::SinkExt;
use log::{debug, info, warn};
use uuid::Uuid;


use crate::peer::PeerMessage;

use super::peer::Command;
use crate::{Message, handle_peer_message, Sender, Result,spawn_and_log_error};


pub async fn accept_loop(addr: impl ToSocketAddrs,broker_sender: Sender<Message>) -> Result<()> {
    info!("Start accepting incomming connections");
    let listener = TcpListener::bind(addr).await?;
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        info!("Accepting from: {}", stream.peer_addr()?);
        spawn_and_log_error(connection_loop(broker_sender.clone(), stream));
    }
    drop(broker_sender);
    Ok(())
}


async fn connection_loop(broker: Sender<Message>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream);
    let addr = stream.peer_addr();
    let reader = BufReader::new(&*stream);

    let mut lines = reader.lines();

    let message = match lines.next().await {
        None => Err("peer disconnected immediately")?,
        Some(line) => line?,
    };

    let addr = match addr {
        Err(..) => Err("Cannot get peer address")?,
        Ok(address) => address.ip().to_string(),
    };

    let (id, client_id, port) = extract_first_message(&message)?;

    debug!("Receive new ConnectClient id :{:?} peer:{:}", id, client_id);
    let mut connection_broker = broker.clone();
    connection_broker.send(Message::NewPeer {id,peer_id: client_id.clone(),address: addr,port,stream: Arc::clone(&stream),})
        .await
        .unwrap();

    let error_threshold = 10;
    
    accept_new_messages(lines, client_id.clone(), error_threshold, broker).await?;

    connection_broker.send(Message::LeavePeer { id: Uuid::new_v4(), peer_id: client_id.clone() })
    .await
    .unwrap();
    Ok(())
}

async fn accept_new_messages(mut lines: async_std::io::Lines<BufReader<&TcpStream>>, client_id: String, error_threshold: i32, mut broker:  Sender<Message>) -> Result<()> {
    let mut error_count = 0;
    Ok(while let Some(line) = lines.next().await {
        let line = match line {
            Err(err) => {
                warn!("Error {:?} reading line from {:?}", err, client_id);
                error_count += 1;
                // End the connection loop, we cannot read any data :/ 
                if error_count == error_threshold {
                    Err("Peer error count reached the threshold")?
                }
                continue;
            }
            Ok(message) => {
                error_count = 0;
                message
            }
        };

        handle_peer_message(line, &mut broker).await?;
    })
}

fn extract_first_message(message: &String) -> Result<(Uuid, String, i32)> {
    let command = match serde_json::from_str(&message) {
        Err(err) => Err(err)?,
        Ok(message) => match message {
            PeerMessage::PeerCommand {command} => command,
            _ => Err("First event wasn't a ClientCommand ")?,
        },
    };
    
    let (id, client_id, port) = match command {
        Command::Connect { id, client_id, port } => (id, client_id, port),
        _ => Err("First event wasn't a Connect command")?,
    };
    Ok((id, client_id, port))
}



