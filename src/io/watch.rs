use std::path::Path;

use log::{debug, error, info, warn};
use notify::{Config, Error, Event, RecommendedWatcher, RecursiveMode, Watcher};
use uuid::Uuid;

use crate::{io::sha, InternalMessage, Result, Sender, broker::InternalToExternal};
use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};




pub async fn async_watch(path: &Path, mut sender: Sender<InternalMessage>) -> Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    watcher.watch(path, RecursiveMode::Recursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => handle_event(event, &mut sender, path).await?,
            Err(e) => handle_error(e).await,
        }
    }

    Ok(())
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

async fn handle_event(
    event: Event,
    sender: &mut Sender<InternalMessage>,
    absolute_root: &Path,
) -> Result<()> {
    let event_id = Uuid::new_v4();
    debug!("{:?} :: Event : {:?}", event_id, event);
    debug!(
        "{:?} :: Create event for file {:?} kind :: {:?}",
        event_id, event.paths, event.kind
    );
    let path = event.paths.get(0).unwrap();
    if !path.exists() {
        debug!("{:?} File {:?} already deleted ", event_id, path);
        return Ok(());
    }
    let asyn_buf = async_std::path::PathBuf::from(path);
    let sha = sha(&asyn_buf).await.unwrap();

    match event.kind {
        notify::EventKind::Create(kind) => match kind {
            notify::event::CreateKind::File => {
                let path = event.paths.get(0).unwrap();

                let relative_path = get_relative_path(absolute_root, path).to_str().unwrap();
                
                let message = InternalToExternal::FileCreated {
                    id: event_id,
                    file: String::from(relative_path),
                    sha,
                };
                sender.send(InternalMessage::InternalToExternal { message }).await?;
            }
            notify::event::CreateKind::Folder => {
                let relative_path = get_relative_path(absolute_root, event.paths.get(0).unwrap())
                    .to_str()
                    .unwrap();
                let message = InternalToExternal::FolderCreated {
                    id: event_id,
                    folder: String::from(relative_path),
                    sha,
                };
                sender.send(InternalMessage::InternalToExternal { message }).await?;
            }
            notify::event::CreateKind::Other => todo!(),
            notify::event::CreateKind::Any => todo!(),
        },
        notify::EventKind::Modify(kind) => match kind {
            notify::event::ModifyKind::Data(_data) => {
                let relative_path = get_relative_path(absolute_root, event.paths.get(0).unwrap())
                    .to_str()
                    .unwrap();
                let message = InternalToExternal::FileModified {
                    id: event_id,
                    file: String::from(relative_path),
                    sha,
                };
                sender.send(InternalMessage::InternalToExternal { message }).await?;
            }
            notify::event::ModifyKind::Name(_name) => {
                let _path = event.paths.get(0).unwrap();
                let _relative_path = get_relative_path(absolute_root, event.paths.get(0).unwrap())
                    .to_str()
                    .unwrap();
            }
            notify::event::ModifyKind::Other | notify::event::ModifyKind::Any => {
                debug!("TODO ignore update for now {:?}",path);
            },
            notify::event::ModifyKind::Metadata(_metadata) => {
                debug!("TODO ignore update for now {:?}",path);
            },
        },
        notify::EventKind::Remove(kind) => match kind {
            notify::event::RemoveKind::File => {}
            notify::event::RemoveKind::Folder => todo!(),
            notify::event::RemoveKind::Any | notify::event::RemoveKind::Other => {
                info!("Remove event for file {:?} kind :: {:?}", event.paths, kind);
            }
        },
        notify::EventKind::Other => {
            warn!("Other event {:?}", event);
        }
        notify::EventKind::Any => {
            warn!("Unknown event {:?}", event);
        }
        notify::EventKind::Access(kind) => {
            info!("Access event for file {:?} kind :: {:?}", event.paths, kind);
        }
    }
    Ok(())
}

fn get_relative_path<'a>(root: &Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap()
}

async fn handle_error(error: Error) {
    error!("watch error: {:?}", error);
}
