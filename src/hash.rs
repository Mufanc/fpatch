use std::path::Path;

pub trait Hash {
    fn hash(&self) -> String;
}

impl Hash for dyn AsRef<[u8]> {
    fn hash(&self) -> String {
        format!("{:x}", md5::compute(self))
    }
}

impl<P : AsRef<Path>> Hash for P {
    fn hash(&self) -> String {
        format!("{:x}", md5::compute(self.as_ref().to_str().unwrap()))
    }
}
