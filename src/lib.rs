extern crate core;

use std::io;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    ObjectError(object::Error),
    MissingElfSection(String),
    ArtifactNotFound,
    BuildFailed(String),
}
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}
impl From<object::Error> for Error {
    fn from(err: object::Error) -> Self {
        Self::ObjectError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod rom;
pub mod elf;