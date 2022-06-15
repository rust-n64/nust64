use std::path::PathBuf;
use clap::{AppSettings, Arg, Command};
use nust64::elf::Elf;
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
        .arg(Arg::new("additional-args")
            .takes_value(true)
            .multiple_values(true))
        .global_setting(AppSettings::DeriveDisplayOrder)
        .next_line_help(true)
        .get_matches();
    
    let project_path = PathBuf::from(matches.value_of("project-path").unwrap_or("."));
    
    let mut ipl3 = std::fs::read(matches.value_of("ipl3").unwrap()).unwrap();
    if ipl3.len() < 4032 {
        if ipl3.len() > 4032 {
            println!("Warning! Provided IPL3 is larger than expected 4032 bytes ({}). IPL3 will be truncated.", ipl3.len())
        } else {
            println!("Warning! Provided IPL3 is smaller than expected 4032 bytes ({}). IPL3 will be padded.", ipl3.len())
        }
        
        ipl3.resize(4032, 0x00);
    }
    
    let elf = match Elf::build(&project_path, Some(&matches.values_of_lossy("additional-args").unwrap_or_default())) {
        Ok(data) => data,
        Err(err) => panic!("Error encountered during build: {:?}", err)
    };
    
    let rom = Rom::new(&elf, ipl3.try_into().unwrap(), None);
    
    std::fs::write(elf.path.with_extension("n64"), rom.to_vec()).unwrap();
}