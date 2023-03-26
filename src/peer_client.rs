extern crate async_std;
extern crate futures;
use std::time::{UNIX_EPOCH, SystemTime};

use async_std::{
    io::{stdin, BufReader},
    net::{TcpStream, ToSocketAddrs},
    prelude::*,
};
use futures::{select, FutureExt};
use uuid::Uuid;
use crate::{peer::PeerMessage,peer::Command};
use crate::{Message, handle_peer_message, Sender, Result};


pub async fn client_connection(addr: impl ToSocketAddrs,mut broker_sender: Sender<Message>) -> Result<()> {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
    let peer_id = format!("client_123_{}",since_the_epoch.subsec_millis());

    let stream = TcpStream::connect(addr).await?;
    let (reader, mut writer) = (&stream, &stream); // 1
    let mut lines_from_server = BufReader::new(reader).lines().fuse(); 
    let mut lines_from_stdin = BufReader::new(stdin()).lines().fuse(); 

    let connect_command = Command::Connect { id: Uuid::new_v4(), client_id: peer_id.clone(), port: 123 }; //TODO set proper UUI for command
    let pessage = PeerMessage::PeerCommand { command: connect_command };
    let command_json = serde_json::to_string(&pessage).unwrap();
    // send_message
    writer.write_all(command_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;

    loop {
        select! { // 3
            line = lines_from_server.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    handle_peer_message(line, &mut broker_sender).await?;
                },
                None => break,
            },
            line = lines_from_stdin.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    
                    let test_command = Command::Test{id: Uuid::new_v4(),peer_id: peer_id.clone(), message: String::from(line)};
                    let test_message = PeerMessage::PeerCommand{ command:  test_command};
                    let command_json = serde_json::to_string(&test_message).unwrap();
                    // send_message
                    writer.write_all(command_json.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                }
                None => break,
            }
        }
    }
    Ok(())
}

