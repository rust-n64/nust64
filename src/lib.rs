extern crate core;

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ArtifactNotFound,
    BuildFailed(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod rom;
pub mod elf;