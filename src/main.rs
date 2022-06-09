use std::path::PathBuf;
use clap::{AppSettings, Arg, Command};
use nust64::rom::Rom;

fn main() {
    let matches = Command::new("nust64")
        .arg(Arg::new("project-path")
            .takes_value(true)
            .help("Path to project you wish to build. If omitted, current directory will be used."))
        .arg(Arg::new("ipl3")
            .takes_value(true)
            .required(true)
            .long("ipl3")
            .help("Path to IPL3 binary file."))
        .global_setting(AppSettings::DeriveDisplayOrder)
        .next_line_help(true)
        .get_matches();
    
    let mut project_path = PathBuf::from(matches.value_of("project-path").unwrap_or("."));
    if project_path.is_dir() {
        project_path.push("Cargo.toml");
        
        if !project_path.is_file() {
            panic!("Project's Cargo.toml file could not be found: {}", project_path.to_string_lossy());
        }
    } else if !project_path.is_file() || !project_path.file_name().unwrap_or_default().eq("Cargo.toml") {
        panic!("Project's Cargo.toml file could not be found: {}", project_path.to_string_lossy());
    }
    
    let mut ipl3 = std::fs::read(matches.value_of("ipl3").unwrap()).unwrap();
    if ipl3.len() < 4032 {
        if ipl3.len() > 4032 {
            println!("Warning! Provided IPL3 is larger than expected 4032 bytes ({}). IPL3 will be truncated.", ipl3.len())
        } else {
            println!("Warning! Provided IPL3 is smaller than expected 4032 bytes ({}). IPL3 will be padded.", ipl3.len())
        }
        
        ipl3.resize(4032, 0x00);
    }
    
    let (elf_data, artifact) = match nust64::build_elf(&project_path, Some(&["--features", "default_tests"])) {
        Ok(data) => data,
        Err(err) => panic!("Error encountered during build: {:?}", err)
    };
    
    let rom = Rom::new(&elf_data, ipl3.try_into().unwrap(), &artifact.file_stem().unwrap_or_default().to_string_lossy());
    
    let output = artifact.with_extension("n64");
    std::fs::write(output, rom.to_vec()).unwrap();
}