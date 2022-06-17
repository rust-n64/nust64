use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use cargo_metadata::Message;
use object::{File, Object, ObjectSection, SectionFlags};
use object::elf::SHF_EXECINSTR;
use crate::{Error::*, Result};

/// Simplified version of an ELF object section.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ElfSection {
    pub addr: u64,
    pub data: Vec<u8>,
}

/// Result of parsing an ELF object file, this stores the important components for generating 
/// a [Rom](crate::rom::Rom).
#[derive(Clone, PartialEq, Debug)]
pub struct Elf {
    pub path: PathBuf,
    pub raw: Vec<u8>,
    pub entry: u32,
    pub sections: HashMap<String, ElfSection>,
    pub is_boot_executable: bool,
}
impl Elf {
    /// Loads an ELF object file, and parses the most critical information from it for use with
    /// this crate. To retrieve additional data, use [`object::File::parse()`].
    pub fn with_file(elf_path: &PathBuf) -> Result<Self> {
        match std::fs::read(elf_path) {
            Ok(raw) => {
                let obj = File::parse(raw.as_slice()).expect(&format!("Failed to parse artifact as ELF: {}", elf_path.to_string_lossy()));
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
                    path: elf_path.clone(),
                    raw,
                    entry,
                    sections,
                    is_boot_executable: (flags & (SHF_EXECINSTR as u64)) != 0
                })
            },
            Err(err) => Err(IoError(err))
        }
    }
    
    /// Executes a cargo build command on the provided project, which should generate
    /// a MIPS-III compatible ELF binary file.
    /// 
    /// Failed attempts to write the LLVM target or linker files necessary for compilation, will cause
    /// a panic. Other errors be returned.
    /// 
    /// Manifest path can either be a path to the project's root directory, or to a project's
    /// Cargo.toml file.
    /// 
    /// Additional arguments can also be provided which will be appended to the build command. Useful
    /// for attaching feature flags or other compile-time arguments needed for the project.
    pub fn build(manifest_path: &PathBuf, additional_args: Option<&[&str]>) -> Result<Self> {
        let mut manifest_path = dunce::canonicalize(manifest_path).unwrap(); // Probably unnecessary, but using it for safety
        if manifest_path.is_dir() {
            manifest_path.push("Cargo.toml");
            
            if !manifest_path.is_file() {
                panic!("Project's Cargo.toml file could not be found: {}", manifest_path.display());
            }
        } else if !manifest_path.is_file() || !manifest_path.file_name().unwrap_or_default().eq("Cargo.toml") {
            panic!("Project's Cargo.toml file could not be found: {}", manifest_path.display());
        }
        
        let mut target_path = manifest_path.parent().unwrap().to_path_buf();
        target_path.push("target/");
        std::fs::create_dir(&target_path).unwrap_or_default();
        target_path.push("mips-nintendo64-none.json");
        
        let linker = include_str!("target-template/linker.ld");
        let linker_path = target_path.with_file_name("linker.ld");
        std::fs::write(&linker_path, linker).unwrap();
        
        let target = include_str!("target-template/mips-nintendo64-none.json").replace("LINKER_PATH", &linker_path.display().to_string().replace("\\", "\\\\"));
        std::fs::write(&target_path, target).unwrap();
        
        
        std::env::set_var("RUSTFLAGS", format!("{} -Clinker-plugin-lto", std::env::var("RUSTFLAGS").unwrap_or_default()).trim());
        
        let toolchain = find_toolchain(&manifest_path).unwrap_or_default();
        if !toolchain.is_empty() && !install_toolchain(&toolchain) {
            println!("Warning: Failed to install rust-src component; build may not succeed.")
        }
        let output = Command::new("cargo")
            .args([
                &format!("+{}", toolchain),
                "build",
                "--release",
                "--manifest-path",
                &manifest_path.display().to_string(),
                "-Z=build-std=core,alloc",
                "--message-format=json-render-diagnostics",
                &format!("--target={}", target_path.display()),
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
                return Self::with_file(&artifact.clone().into_std_path_buf());
            }
            
            Err(ArtifactNotFound)
        } else {
            Err(BuildFailed(format!("cargo build failed: {}", output.status)))
        }
    }
}

fn find_toolchain(manifest_path: &PathBuf) -> Option<String> {
    let parent_path = match manifest_path.parent() {
        Some(path) => path.to_path_buf(),
        None => return None
    };
    
    let mut legacy_path = parent_path.clone();
    legacy_path.push("rust-toolchain");
    if legacy_path.is_file() {
        return Some(std::fs::read_to_string(legacy_path).unwrap_or_default().trim().to_owned());
    }
    
    let mut toml_path = parent_path.clone();
    toml_path.push("rust-toolchain.toml");
    if toml_path.is_file() {
        let contents = std::fs::read_to_string(&toml_path).unwrap_or_default();
        
        if contents.contains("channel =") {
            #[derive(serde::Deserialize, Debug)]
            struct ToolchainToml {
                toolchain: ToolchainTable,
            }
            #[derive(serde::Deserialize, Debug)]
            struct ToolchainTable {
                channel: String,
            }
            
            let toml: ToolchainToml = toml::from_str(&contents).expect(&format!("Failed to parse as TOML: {}", toml_path.to_string_lossy()));
            return Some(toml.toolchain.channel)
        }
    }
    
    None
}

fn install_toolchain(toolchain: &str) -> bool {
    let output = Command::new("rustup")
        .args([
            "install",
            toolchain,
        ])
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    if !output.status.success() {
        return false;
    }
    
    let output = Command::new("rustup")
        .args([
            "run",
            toolchain,
            "--",
            "rustup",
            "component",
            "add",
            "rust-src",
        ])
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    
    output.status.success()
}