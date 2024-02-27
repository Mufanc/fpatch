use std::{cmp, fs};
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{bail, Result};
use fuser::{FileAttr, Filesystem, FileType, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen, Request};
use libc::*;
use log::debug;
use once_cell::unsync::Lazy;
use rustix::{fs as rfs, process};
use rustix::process::Signal;

use crate::{check_ns, dirs};
use crate::configs::{PatchedFile, PatchType};
use crate::dirs::{FileNameString, MOUNT_POINT};
use crate::hash::Hash;

const TTL: Duration = Duration::from_secs(1);

const ROOT_INO: u64 = 1;  // fuse root
const ROOT_ATTR: Lazy<FileAttr> = Lazy::new(|| FileAttr {
    ino: ROOT_INO,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 0,
    uid: process::getuid().as_raw(),
    gid: process::getgid().as_raw(),
    rdev: 0,
    blksize: 0,
    flags: 0,
});


fn generate_attr(file: &PatchedFile) -> FileAttr {
    let path = &file.path;
    let src = rfs::stat(path).unwrap();

    FileAttr {
        ino: src.st_ino,
        size: match file.patch_type {
            PatchType::Replace => src.st_size as _,
            PatchType::Prepend | PatchType::Append => {
                src.st_size as u64 + file.content.len() as u64
            }
        },
        blocks: src.st_blocks as _,
        atime: UNIX_EPOCH + Duration::new(src.st_atime as _, src.st_atime_nsec as _),
        mtime: UNIX_EPOCH + Duration::new(src.st_mtime as _, src.st_mtime_nsec as _),
        ctime: UNIX_EPOCH + Duration::new(src.st_ctime as _, src.st_ctime_nsec as _),
        crtime: UNIX_EPOCH,  // mac only
        kind: FileType::RegularFile,  // only regular files are supported
        perm: (src.st_mode & 0o777) as _,
        nlink: src.st_nlink as _,
        uid: src.st_uid,
        gid: src.st_gid,
        rdev: src.st_rdev as _,
        blksize: src.st_blksize as _,
        flags: 0,  // mac only
    }
}


pub struct FuseEntry {
    name: String,
    attr: FileAttr,
    src: Option<PatchedFile>
}

impl FuseEntry {
    fn new(name: String, attr: FileAttr, file: Option<PatchedFile>) -> Self {
        return Self { name, attr, src: file }
    }

    fn specials() -> Vec<Self> {
        vec![
            Self::new(".".to_owned(), *ROOT_ATTR, None),
            Self::new("..".to_owned(), *ROOT_ATTR, None)
        ]
    }
}

impl From<PatchedFile> for FuseEntry {
    fn from(file: PatchedFile) -> Self {
        let filepath = &file.path;

        FuseEntry::new(
            format!("{}:{}", filepath.hash(), filepath.name_string()),
            generate_attr(&file),
            Some(file)
        )
    }
}


struct MirrorFileSystem {
    entries: Vec<FuseEntry>
}

impl MirrorFileSystem {
    fn new(files: Vec<PatchedFile>) -> Self {
        let mut entries = FuseEntry::specials();
        entries.extend(files.into_iter().map(FuseEntry::from));
        
        Self { entries }
    }
}

impl Filesystem for MirrorFileSystem {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent != ROOT_INO {
            reply.error(ENOENT);
            return;
        }

        let name = name.to_str().unwrap();
        let entry = self.entries.iter().find(|entry| {
            entry.name == name
        });

        if let Some(entry) = entry {
            reply.entry(&TTL, &entry.attr, 0);
        } else {
            reply.error(ENOENT)
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        if ino == ROOT_INO {
            reply.attr(&TTL, &ROOT_ATTR);
            return;
        }

        let entry = self.entries.iter().find(|entry| entry.attr.ino == ino);

        if let Some(entry) = entry {
            reply.attr(&TTL, &entry.attr);
        } else {
            reply.error(ENOENT)
        }
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        let entry = self.entries.iter().find(|entry| entry.attr.ino == ino);

        if let Some(FuseEntry { src: Some(_), .. }) = entry {
            reply.opened(ino, 0);
        } else {
            reply.error(EINVAL)
        }
    }

    fn read(&mut self, _req: &Request<'_>, ino: u64, _fh: u64, offset: i64, size: u32, _flags: i32, _lock_owner: Option<u64>, reply: ReplyData) {
        let entry = self.entries.iter().find(|entry| entry.attr.ino == ino);

        if let Some(entry) = entry {
            let file = entry.src.as_ref().unwrap();

            if let Ok(data) = do_read(file, offset as _, size as _, entry.attr.size as _) {
                reply.data(&data);
            } else {
                reply.error(EIO);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(&mut self, _req: &Request<'_>, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino != ROOT_INO {
            reply.error(ENOENT);
            return;
        }

        for (i, entry) in self.entries.iter().skip(offset as _).enumerate() {
            if reply.add(entry.attr.ino, (i + 1) as _, entry.attr.kind, entry.name.clone()) {
                break;
            }
        }

        reply.ok();
    }
}


#[derive(Debug)]
struct FileRegion {
    s_begin: usize,
    s_end: usize,
    d_begin: usize,
    d_end: usize
}

impl FileRegion {
    fn s_size(&self) -> usize {
        self.s_end - self.s_begin
    }

    fn d_size(&self) -> usize {
        self.d_end - self.d_begin
    }
}

fn do_read(file: &PatchedFile, begin: usize, size: usize, max_index: usize) -> Result<Vec<u8>> {
    let end = cmp::min(begin + size, max_index);

    let data = &file.content;

    let s_size = fs::metadata(&file.path)?.size() as usize;
    let d_size = data.len();

    let region = match file.patch_type {
        PatchType::Prepend => FileRegion {
            s_begin: cmp::max(begin, d_size) - d_size,
            s_end: cmp::max(end, d_size) - d_size,
            d_begin: cmp::min(begin, d_size),
            d_end: cmp::min(end, d_size)
        },
        PatchType::Append => FileRegion {
            s_begin: cmp::min(begin, s_size),
            s_end: cmp::min(end, s_size),
            d_begin: cmp::max(begin, s_size) - s_size,
            d_end: cmp::max(end, s_size) - s_size,
        },
        PatchType::Replace => FileRegion {
            s_begin: 0,
            s_end: 0,
            d_begin: 0,
            d_end: d_size,
        }
    };

    let mut src_buffer: Vec<u8> = vec![];
    let mut data_buffer: Vec<u8> = vec![];

    if region.s_size() != 0 {
        let fp = File::open(&file.path)?;
        src_buffer.resize(region.s_size(), 0);
        fp.read_exact_at(&mut src_buffer, region.s_begin as _)?;
    }

    if region.d_size() != 0 {
        data_buffer.extend(&data[region.d_begin..region.d_end]);
    }

    Ok(match file.patch_type {
        PatchType::Prepend => {
            data_buffer.extend(src_buffer);
            data_buffer
        }
        PatchType::Append => {
            src_buffer.extend(data_buffer);
            src_buffer
        }
        PatchType::Replace => data_buffer
    })
}


pub fn mount(files: Vec<PatchedFile>) -> Result<()> {
    let mfs = MirrorFileSystem::new(files);
    let options = &[MountOption::AutoUnmount, MountOption::AllowOther, MountOption::RO];
    
    dirs::ensure_dir(MOUNT_POINT.as_path())?;

    let session = fuser::spawn_mount2(mfs, MOUNT_POINT.as_path(), options)?;
    
    debug!("fuse session: {session:?}");
    
    check_ns()?;
    
    process::kill_process(process::getppid().unwrap(), Signal::Usr1)?;

    match session.guard.join() {
        Err(e) => bail!("fuse mount crashed: {e:?}"),
        _ => bail!("fuse mount exited unexpectedly")
    }
}
