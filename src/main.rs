use std::path::PathBuf;
use clap::{AppSettings, Arg, Command, crate_version};
use nust64::elf::Elf;
use nust64::rom::Rom;

fn main() {
    let matches = Command::new("nust64")
        .version(crate_version!())
        .arg(Arg::new("project-path")
            .takes_value(true)
            .short('p')
            .long("path")
            .help("Path to project you wish to build. If omitted, current directory will be used."))
        .arg(Arg::new("elf-path")
            .takes_value(true)
            .long("elf")
            .help("Path to an already compiled ELF, to be converted into an N64 ROM. If included, anything compiled using the `project-path` argument will be ignored."))
        .arg(Arg::new("ipl3")
            .takes_value(true)
            .required(true)
            .long("ipl3")
            .help("Path to IPL3 binary file."))
        .arg(Arg::new("additional-args")
            .takes_value(true)
            .multiple_values(true)
            .allow_invalid_utf8(true)
            .allow_hyphen_values(true))
        .global_setting(AppSettings::DeriveDisplayOrder)
        .next_line_help(true)
        .get_matches();
    
    let project_path = PathBuf::from(matches.value_of("project-path").unwrap_or("."));
    
    let mut ipl3 = std::fs::read(matches.value_of("ipl3").unwrap()).unwrap();
    if ipl3.len() != 4032 {
        if ipl3.len() > 4032 {
            println!("Warning! Provided IPL3 is larger than expected 4032 bytes ({}). IPL3 will be truncated.", ipl3.len())
        } else {
            println!("Warning! Provided IPL3 is smaller than expected 4032 bytes ({}). IPL3 will be padded.", ipl3.len())
        }
        
        ipl3.resize(4032, 0x00);
    }
    
    let elf = match matches.value_of("elf-path") {
        Some(path) => match Elf::with_file(&PathBuf::from(path)) {
            Ok(data) => data,
            Err(err) => panic!("Error encountered while loading ELF file: {:?}", err)
        },
        None => match Elf::build(&project_path, Some(&matches.values_of_lossy("additional-args").unwrap_or_default().iter().map(|s| s.as_str()).collect::<Vec<&str>>())) {
            Ok(data) => data,
            Err(err) => panic!("Error encountered during build: {:?}", err)
        }
    };
    
    let rom = Rom::new(&elf, ipl3.try_into().unwrap(), None);
    
    let rom_path = elf.path.with_extension("z64");
    std::fs::write(&rom_path, rom.to_vec()).unwrap();
    println!("Generated ROM at: {}", rom_path.canonicalize().unwrap_or(rom_path).display());
}