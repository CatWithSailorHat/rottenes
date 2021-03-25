use std::io;

#[derive(Debug)]
pub enum LoadError {
    NotNesRom,
    IoError(io::Error),
    UnsupportedMapper(u16),
}

impl From<io::Error> for LoadError {
    fn from(e: io::Error) -> LoadError {
        LoadError::IoError(e)
    }
}