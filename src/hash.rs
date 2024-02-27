pub trait Hash {
    fn hash(&self) -> String;
}

impl<T : AsRef<[u8]>> Hash for T {
    fn hash(&self) -> String {
        format!("{:x}", md5::compute(self))
    }
}
