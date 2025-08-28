use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Clone, Debug)]
pub struct TryDir {
    pub basename: String,
    pub path: PathBuf,
    pub ctime: Option<SystemTime>,
    pub mtime: Option<SystemTime>,
    pub score: f64,
}
