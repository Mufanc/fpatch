use crate::dirs::{ensure_dir, ROOT_DIR};

mod fuse;
mod configs;
mod dirs;

fn main() {
    env_logger::init();
    ensure_dir(ROOT_DIR.as_path());
    fuse::mount(configs::parse());
}
