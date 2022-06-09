extern crate core;

use std::path::PathBuf;
use std::process::{Command, Stdio};
use cargo_metadata::Message;

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ArtifactNotFound,
    BuildFailed,
}
use Error::*;

pub type Result<T> = std::result::Result<T, Error>;

pub mod rom;


pub fn build_elf(manifest_path: &PathBuf, additional_args: Option<&[&str]>) -> Result<(Vec<u8>, PathBuf)> {
    let manifest_path = manifest_path.canonicalize().unwrap();
    
    let mut target_path = manifest_path.parent().unwrap().to_path_buf();
    target_path.push("target/");
    std::fs::create_dir(&target_path).unwrap_or_default();
    target_path.push("mips-nintendo64-none.json");
    
    let linker = include_str!("target-template/linker.ld");
    let linker_path = target_path.with_file_name("linker.ld");
    std::fs::write(&linker_path, linker).unwrap();
    
    let target = include_str!("target-template/mips-nintendo64-none.json").replace("LINKER_PATH", &linker_path.to_string_lossy());
    std::fs::write(&target_path, target).unwrap();
    
    
    std::env::set_var("RUSTFLAGS", format!("{} -Clinker-plugin-lto", std::env::var("RUSTFLAGS").unwrap_or_default()).trim());
    
    let output = Command::new("cargo")
        .args([
            "+nightly-2022-03-27", //TODO pull this dynamically from manifest's directory
            "build",
            "--release",
            "--manifest-path",
            &manifest_path.to_string_lossy(),
            "-Z=build-std=core,alloc",
            "--message-format=json-render-diagnostics",
            &format!("--target={}", &target_path.to_string_lossy()),
        ])
        .args(additional_args.unwrap_or_default())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    
    if output.status.success() {
        let mut artifacts = vec![];
        for message in cargo_metadata::Message::parse_stream(output.stdout.as_slice()) {
            if let Ok(message) = message {
                match message {
                    Message::CompilerArtifact(artifact) => {
                        if let Some(path) = artifact.executable {
                            artifacts.push(path);
                        }
                    }
                    _ => (),
                }
            }
        }
        
        if let Some(artifact) = artifacts.last() {
            return match std::fs::read(artifact) {
                Ok(data) => Ok((data, artifact.clone().into_std_path_buf())),
                Err(err) => Err(IoError(err))
            }
        }
        
        Err(ArtifactNotFound)
    } else {
        Err(BuildFailed)
    }
}