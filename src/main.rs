use std::{env, path::Path, time::{SystemTime, UNIX_EPOCH}};

use async_std::task;
use decen_peer::{peer_server::accept_loop, broker_loop, file_check::async_watch, rendezvous_client::server_connection_loop, get_available_port};
use futures::channel::mpsc;


fn main() {
    let args: Vec<String> = env::args().collect();
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    
    let (broker_sender, broker_receiver) = mpsc::unbounded();

    let available_port = match get_available_port() {
        Some(available_port) => available_port,
        None => 9000,
    };

    
    let peer_id = peer_id();
    let rendezvous_server_connection_hander =  server_connection_loop("127.0.0.1:8080",&peer_id,broker_sender.clone(),available_port.into());

    let accept_address = format!("127.0.0.1:{}",available_port);
    let server_handler = accept_loop(accept_address.as_str(),broker_sender.clone());

    let file_watch_handler = match args.get(1) {
        Some(path) => {
            let path  = Path::new(path);
            async_watch(path,broker_sender.clone())
        },
        None => {
            let path  = Path::new("/Users/kasunranasinghe/Development/RUST/test");
            async_watch(path,broker_sender.clone())
        },
    } ;

    let broker_handle = broker_loop(broker_receiver);
    let joined_futures = futures::future::join4(rendezvous_server_connection_hander,server_handler,broker_handle,file_watch_handler);
    let _result = task::block_on(joined_futures);

}

fn peer_id() -> String {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
    let peer_id = format!("client_123_{}",since_the_epoch.subsec_millis());
    peer_id
}
