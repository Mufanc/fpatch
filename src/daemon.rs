use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use log::{debug, info};
use rustix::fs::UnmountFlags;
use rustix::mount;
use tokio::{select, task, time};
use tokio::signal::unix;
use tokio::signal::unix::SignalKind;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::{cli, configs};
use crate::cli::OperationType;
use crate::configs::PatchedFile;
use crate::dirs::{FileNameString, MOUNT_POINT};
use crate::extensions::{Also, ToTokioCommand};
use crate::hash::Hash;

pub async fn main() -> Result<()> {
    crate::mount::cleanup()?;

    loop {
        run_fuse().await?;

        info!("fuse server exited, restarting in 5 seconds...");
        time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run_fuse() -> Result<()> {
    let files_1 = Arc::new(configs::parse());
    let files_2 = files_1.clone();

    let mut fuse = cli::run_op(OperationType::MountFuse)
        .tokio()
        .spawn()?;
    
    let fuse_pid = fuse.id().unwrap();

    let (tx, rx) = oneshot::channel::<()>();

    let do_mount: JoinHandle<Result<()>> = task::spawn(async move {
        try {
            let mut handler = unix::signal(SignalKind::user_defined1())?;

            handler.recv().await.also(|_| debug!("fuse mounted"));

            cli::run_op(OperationType::PipeBack)
                .arg(format!("{}", fuse_pid))
                .status()?;
            
            mount_proxies(&files_1)?;

            rx.await?;
        }
    });

    let run_daemon: JoinHandle<Result<()>> = task::spawn(async move {
        try {
            fuse.wait().await?;
            tx.send(()).unwrap_or_else(|_| ());
            restore_all(&files_2)?;
        }
    });

    select! {
        r = do_mount => debug!("do_mount finished: {r:?}"),
        r = run_daemon => debug!("run_daemon finished: {r:?}")
    }

    crate::mount::cleanup()?;

    Ok(())
}

fn mount_proxies(patches: &[PatchedFile]) -> Result<()> {
    let mut entries: HashMap<String, PathBuf> = HashMap::new();

    for entry in fs::read_dir(&*MOUNT_POINT)? {
        let path = entry.unwrap().path();

        let filename = path.name_string();
        let hash = filename.split(':').next().unwrap().to_string();

        entries.insert(hash, path);
    }

    for file in patches {
        let target = &file.path;
        let source = &entries[&target.hash()];

        crate::mount::bind_mount(source, target)?;
    }

    Ok(())
}

fn restore_all(patches: &[PatchedFile]) -> Result<()> {
    for file in patches {
        mount::unmount(&file.path, UnmountFlags::DETACH)?;
    }

    Ok(())
}
