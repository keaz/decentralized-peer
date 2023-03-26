use std::{env, path::Path};

use async_std::task;
use decen_peer::{peer_server::accept_loop, peer_client::client_connection, broker_loop, file_check::async_watch};
use futures::channel::mpsc;


fn main() {
    let args: Vec<String> = env::args().collect();
    
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    
    let (broker_sender, broker_receiver) = mpsc::unbounded();
    let server_handler = match args.get(1) {
        Some(addr) => accept_loop(addr.as_str(),broker_sender.clone()),
        None => accept_loop("127.0.0.1:8080",broker_sender.clone()),
    }; 

    let file_watch_handler = match args.get(3) {
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

    match args.get(2) {
        None => {
            let joined_futures = futures::future::join3(server_handler,broker_handle,file_watch_handler);
            let _result = task::block_on(joined_futures);
        },
        Some(addr) => {
            let joined_futures  = futures::future::join4(server_handler,broker_handle,client_connection(addr, broker_sender),file_watch_handler);
            let _result = task::block_on(joined_futures);
        },
    }

}
