use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use cargo_metadata::Message;
use object::{File, Object, ObjectSection, SectionFlags};
use object::elf::SHF_EXECINSTR;
use crate::{Error::*, Result};

#[derive(Clone, PartialEq, Debug, Default)]
pub struct ElfSection {
    pub addr: u64,
    pub data: Vec<u8>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Elf {
    pub path: PathBuf,
    pub raw: Vec<u8>,
    pub entry: u32,
    pub sections: HashMap<String, ElfSection>,
    pub is_boot_executable: bool,
}
impl Elf {
    pub fn build(manifest_path: &PathBuf, additional_args: Option<&[impl AsRef<str>]>) -> Result<Self> {
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
            .args(additional_args.unwrap_or_default().iter().map(|s| s.as_ref()).collect::<Vec<&str>>())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();
        
        if output.status.success() {
            let mut artifacts = vec![];
            for message in Message::parse_stream(output.stdout.as_slice()) {
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
            
            // The ELF binary should be the last artifact in the list, if not the only one
            if let Some(artifact) = artifacts.last() {
                return match std::fs::read(artifact) {
                    Ok(raw) => {
                        let obj = File::parse(raw.as_slice()).expect(&format!("Failed to parse artifact as ELF: {}", artifact));
                        let entry = obj.entry() as u32;
                        let boot = obj.section_by_name(".boot").expect(".boot section not found!");
                        let flags = match boot.flags() {
                            SectionFlags::Elf { sh_flags } => sh_flags,
                            _ => 0
                        };
                        
                        let mut sections = HashMap::new();
                        for section in obj.sections() {
                            sections.insert(section.name().unwrap_or(&format!("{:#0X}", section.address())).to_owned(), ElfSection {
                                addr: section.address(),
                                data: section.data().unwrap_or_default().to_vec()
                            });
                        }
                        
                        Ok(Self {
                            path: artifact.clone().into_std_path_buf(),
                            raw,
                            entry,
                            sections,
                            is_boot_executable: (flags & (SHF_EXECINSTR as u64)) != 0
                        })
                    },
                    Err(err) => Err(IoError(err))
                }
            }
            
            Err(ArtifactNotFound)
        } else {
            Err(BuildFailed(format!("cargo build failed: {}", output.status)))
        }
    }
}