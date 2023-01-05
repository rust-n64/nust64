use std::fs;
use std::path::PathBuf;
use std::process::Command;
use bpaf::Bpaf;
use shlex::Shlex;
use nust64::elf::Elf;
use nust64::rom::Rom;

//TODO:
// - insert file at specific location (extending ROM if necessary)

/// nust64 - ELF binary to N64 ROM converter
#[derive(Debug, Bpaf)]
#[bpaf(options, version, generate(args))]
struct Args {
    /// command to execute before ROM generation
    #[bpaf(long("pre-exec"))]
    pre_exec: Vec<String>,
    
    /// command to execute after ROM generation (e.g. running an emulator)
    /// 
    /// Note: any instance of `>>ROM<<` in a command string, will be replaced with the generated ROM's path
    #[bpaf(long("post-exec"))]
    post_exec: Vec<String>,
    
    /// name of ELF section to include in ROM (if omitted, included sections are: .boot, .text, .rodata, .data, .assets, and .bss)
    #[bpaf(short, long("section"))]
    sections: Vec<String>,
    
    /// append file to generated ROM
    #[bpaf(short, long("append"))]
    appends: Vec<PathBuf>,
    
    /// name to put in ROM header (max 20 bytes)
    #[bpaf(short, long)]
    name: Option<String>,
    
    /// path to IPL3 binary (if none, IPL3 section will be filled with 0x00)
    #[bpaf(long)]
    ipl3: Option<PathBuf>,
    
    /// path to ELF file
    #[bpaf(long)]
    elf: PathBuf,
}

fn main() {
    let args = args().run();
    
    for pre in args.pre_exec {
        exec(&pre);
    }
    
    let mut ipl3 = match args.ipl3 {
        Some(ref path) => fs::read(path).expect(&format!("IPL3 does not exist: {}", path.display())),
        None => vec![0; 4032],
    };
    if ipl3.len() != 4032 {
        if ipl3.len() > 4032 {
            println!("Warning! Provided IPL3 is larger than expected 4032 bytes ({}). IPL3 will be truncated.", ipl3.len())
        } else {
            println!("Warning! Provided IPL3 is smaller than expected 4032 bytes ({}). IPL3 will be padded.", ipl3.len())
        }
        
        ipl3.resize(4032, 0x00);
    }
    
    let elf = Elf::new(&args.elf).expect("failed to parse ELF");
    
    let mut rom = Rom::new(&elf, ipl3.try_into().unwrap(), args.name, args.sections);
    let rom_path = elf.path.with_extension("z64");
    
    let data = &mut rom.binary;
    for append in args.appends {
        if append.is_file() {
            data.extend_from_slice(&fs::read(append).unwrap());
        }
    }
    
    fs::write(&rom_path, rom.to_vec()).unwrap();
    let rom_path = rom_path.canonicalize().unwrap_or(rom_path);
    println!("Generated ROM at: {}", rom_path.display());
    
    for post in args.post_exec {
        exec(&post.replace(">>ROM<<", rom_path.display().to_string().as_str()));
    }
}

fn exec(cmd_str: &str) {
    let mut lex = Shlex::new(cmd_str);
    let args = lex.by_ref().collect::<Vec<_>>();
    if args.is_empty() || lex.had_error { return; }
    
    Command::new(&args[0])
        .args(&args[1..])
        .spawn()
        .expect(&format!("failed to start exec: {cmd_str}"))
        .wait()
        .expect(&format!("failed to wait for exec: {cmd_str}"));
}