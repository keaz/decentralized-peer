pub mod file_handler;
use std::path::Path;

use log::{error, info, warn};
use notify::{Watcher, RecommendedWatcher, RecursiveMode,Event,Error, Config};
use uuid::Uuid;

use crate::{Sender, Message,Result};
use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};


pub async fn async_watch(path: &Path,mut sender: Sender<Message>) -> Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    watcher.watch(path, RecursiveMode::Recursive)?;
    
    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => handle_event(event,&mut sender, path).await?,
            Err(e) => handle_error(e).await,
        }
    }

    Ok(())
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    let watcher = RecommendedWatcher::new(move |res| {
        futures::executor::block_on(async {
            tx.send(res).await.unwrap();
        })
    }, Config::default())?;

    Ok((watcher, rx))
}


async fn handle_event(event: Event,sender: &mut Sender<Message>, absolute_root: &Path) -> Result<()> {
    match event.kind {
        notify::EventKind::Create(kind) => {
            match kind {
                notify::event::CreateKind::File => {
                    let relative_path = get_relative_path(absolute_root,event.paths.get(0).unwrap()).to_str().unwrap();
                    let message = Message::FileCreated { id: Uuid::new_v4(), file: String::from(relative_path) };
                    sender.send(message).await?;
                },
                notify::event::CreateKind::Folder => {
                    let relative_path = get_relative_path(absolute_root,event.paths.get(0).unwrap()).to_str().unwrap();
                    let message = Message::FolderCreated { id: Uuid::new_v4(), folder: String::from(relative_path) };
                    sender.send(message).await?;
                },
                notify::event::CreateKind::Other => todo!(),
                notify::event::CreateKind::Any => todo!(),
            }
        },
        notify::EventKind::Modify(kind) => {
            match kind {
                notify::event::ModifyKind::Data(data) => {
                    let relative_path = get_relative_path(absolute_root,event.paths.get(0).unwrap()).to_str().unwrap();
                    let message = Message::FileCreated { id: Uuid::new_v4(), file: String::from(relative_path) };
                    sender.send(message).await?;
                },
                notify::event::ModifyKind::Name(name) => {
                    let relative_path = get_relative_path(absolute_root,event.paths.get(0).unwrap()).to_str().unwrap();
                    let message = Message::FolderCreated { id: Uuid::new_v4(), folder: String::from(relative_path) };
                    sender.send(message).await?;
                },
                notify::event::ModifyKind::Other | notify::event::ModifyKind::Any => todo!(),
                notify::event::ModifyKind::Metadata(metadata) => todo!(),
            }
        },
        notify::EventKind::Remove(kind) => {
            match kind {
                notify::event::RemoveKind::File => todo!(),
                notify::event::RemoveKind::Folder => todo!(),
                notify::event::RemoveKind::Any | notify::event::RemoveKind::Other => {
                    info!("Remove event for file {:?} kind :: {:?}",event.paths,kind);
                },
            }
        },
        notify::EventKind::Other => {
            warn!("Other event {:?}",event);
        },
        notify::EventKind::Any => {
            warn!("Unknown event {:?}",event);
        },
        notify::EventKind::Access(kind) => {
            info!("Access event for file {:?} kind :: {:?}",event.paths,kind);
        },
    }
    Ok(())
}

fn get_relative_path<'a>(root: &Path, path : &'a Path) -> &'a Path{
    path.strip_prefix(root).unwrap()
}

async fn handle_error(error: Error) {
    error!("watch error: {:?}", error);
}





