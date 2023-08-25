use std::{
    env,
    path::Path,
    time::{SystemTime, UNIX_EPOCH}, rc::Rc,
};

use async_std::task;
use clap::Parser;
use decen_peer::{
    broker::Broker, cmd::CmdArgs, get_available_port, io::{watch::async_watch, file_handler::FileHandler},
    rendezvous::Server, server::PeerServer, peer::client::ClientConnectionHandler, PeerMessageHandler,
};
use futures::channel::mpsc;


fn main() {
    let _args: Vec<String> = env::args().collect();
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    let cmds = CmdArgs::parse_from(env::args_os());
    let (broker_sender, broker_receiver) = mpsc::unbounded();

    let available_port = match get_available_port() {
        Some(available_port) => available_port,
        None => 9000,
    };

    let folder = cmds.folder.clone();
    let path = Path::new(folder.as_str());

    let peer_message_hander = Rc::new(PeerMessageHandler::new());
    
    let client_handler = ClientConnectionHandler::new(peer_message_hander.clone());
    let server = Server::new(client_handler);
    let peer_id = peer_id();
    let rendezvous_server_connection_hander = server.server_connection_loop(
        "127.0.0.1:8080",
        &peer_id,
        broker_sender.clone(),
        available_port.into(),
    );

    let accept_address = format!("127.0.0.1:{}", available_port);
    let peer_server = PeerServer::new(peer_message_hander.clone());
    let server_handler =  peer_server.accept_loop(accept_address.as_str(), broker_sender.clone());
    
    let file_watch_handler = async_watch(path, broker_sender.clone());
    let file_handler = FileHandler::new(cmds.folder);
    let broker = Broker::new(peer_id.clone(),file_handler);
    let broker_handle = broker.broker_loop(broker_receiver);
    let joined_futures = futures::future::join4(
        rendezvous_server_connection_hander,
        server_handler,
        broker_handle,
        file_watch_handler,
    );
    let _result = task::block_on(joined_futures);
}

fn peer_id() -> String {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let peer_id = format!("client_123_{}", since_the_epoch.subsec_millis());
    peer_id
}
