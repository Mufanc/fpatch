use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use fuser::{FileAttr, Filesystem, FileType, MountOption, ReplyAttr, ReplyDirectory, ReplyEntry, ReplyXattr, Request};
use log::debug;
use nix::libc::*;
use nix::sys::stat;

use crate::configs::{PatchConfigsModel, PatchModel};
use crate::dirs::{ensure_dir, MOUNT_POINT};

const TTL: Duration = Duration::from_secs(1);

const DIR_INO: u64 = 1;  // fuse root
const DIR_ATTR: FileAttr = FileAttr {
    ino: DIR_INO,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 0,
    uid: 0,
    gid: 0,
    rdev: 0,
    blksize: 0,
    flags: 0,
};


#[derive(Debug, Copy, Clone)]
enum PatchType {
    Prepend,
    Append,
    Replace
}

#[derive(Debug)]
struct PatchConfig {
    file: PathBuf,
    name: String,
    content: String,
    ptype: PatchType,
    attr: FileAttr
}

struct FileSystemEntry {
    ino: u64,
    ftype: FileType,
    name: String
}

impl FileSystemEntry {
    fn new(ino: u64, ftype: FileType, name: &str) -> Self {
        return FileSystemEntry {
            ino, ftype,
            name: name.to_owned()
        }
    }
}


struct MirrorFileSystem {
    configs: Vec<PatchConfig>
}

impl Filesystem for MirrorFileSystem {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent != DIR_INO {
            reply.error(ENOENT);
            return;
        }

        let name = name.to_str().unwrap();
        let config = self.configs.iter().find(|config| {
            config.name == name
        });
        
        debug!("lookup {name}: {config:?}");

        if let Some(config) = config {
            reply.entry(&TTL, &config.attr, 0);
        } else {
            reply.error(ENOENT)
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        if ino == DIR_INO {
            reply.attr(&TTL, &DIR_ATTR);
            return;
        }

        let config = self.configs.iter().find(|config| config.attr.ino == ino);
        debug!("getattr for {ino}: {config:?}");
        
        if let Some(config) = config {
            reply.attr(&TTL, &config.attr);
        } else {
            reply.error(ENOENT)
        }
    }

    fn readdir(&mut self, _req: &Request<'_>, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino != DIR_INO {
            reply.error(ENOENT);
            return;
        }
        
        debug!("readdir: offset={offset}");

        let entries = merge_entries(&self.configs);

        for (i, entry) in entries.iter().skip(offset as _).enumerate() {
            if reply.add(entry.ino, (i + 1) as _, entry.ftype, entry.name.clone()) {
                break;
            }
        }

        reply.ok();
    }
}


fn copy_attr<P : AsRef<Path>>(file: P) -> FileAttr {
    let filename = file.as_ref().to_str().unwrap().to_owned();
    let src = stat::stat(file.as_ref()).unwrap_or_else(|err| panic!("failed to stat file {}: {}", filename, err));

    FileAttr {
        ino: src.st_ino,
        size: src.st_size as _,
        blocks: src.st_blocks as _,
        atime: UNIX_EPOCH + Duration::new(src.st_atime as _, src.st_atime_nsec as _),
        mtime: UNIX_EPOCH + Duration::new(src.st_mtime as _, src.st_mtime_nsec as _),
        ctime: UNIX_EPOCH + Duration::new(src.st_ctime as _, src.st_ctime_nsec as _),
        crtime: UNIX_EPOCH,  // mac only
        kind: FileType::NamedPipe,
        perm: src.st_mode as _,
        nlink: src.st_nlink as _,
        uid: src.st_uid,
        gid: src.st_gid,
        rdev: src.st_rdev as _,
        blksize: src.st_blksize as _,
        flags: 0,  // mac only
    }
}


fn transform_configs(configs: PatchConfigsModel) -> Vec<PatchConfig> {
    let mut result = vec![];
    let mut transform = |ptype: PatchType, models: Vec<PatchModel>| {
        models.into_iter().for_each(|model| {
            let file = PathBuf::from(&model.file);
            let filename = file.file_name().and_then(|oss| oss.to_str())
                .unwrap_or_else(|| panic!("failed to get filename for {}", model.file))
                .to_owned();

            result.push(PatchConfig {
                file,
                name: filename,
                content: model.content,
                ptype,
                attr: copy_attr(&model.file)
            });
        });
    };

    if let Some(models) = configs.prepend {
        transform(PatchType::Prepend, models);
    }

    if let Some(models) = configs.append {
        transform(PatchType::Append, models);
    }

    if let Some(models) = configs.replace {
        transform(PatchType::Replace, models);
    }

    return result
}

fn merge_entries(configs: &[PatchConfig]) -> Vec<FileSystemEntry> {
    let mut entries = vec![];

    entries.push(FileSystemEntry::new(DIR_INO, DIR_ATTR.kind, "."));
    entries.push(FileSystemEntry::new(DIR_INO, DIR_ATTR.kind, ".."));

    entries.extend(configs.iter().map(|config| {
        let attr = config.attr;
        FileSystemEntry::new(attr.ino, attr.kind, &config.name)
    }));

    return entries;
}


pub fn mount(configs: PatchConfigsModel) {
    let configs = transform_configs(configs);
    let mfs = MirrorFileSystem { configs };
    // let options = &[MountOption::AutoUnmount, MountOption::AllowRoot]; 
    
    ensure_dir(MOUNT_POINT.as_path());
    fuser::mount2(mfs, MOUNT_POINT.as_path(), &[]).expect("failed to mount mirror");
}
