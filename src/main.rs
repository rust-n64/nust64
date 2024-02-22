use std::fs;
use std::process::Command;
use std::str::FromStr;
use bpaf::Bpaf;
use camino::{Utf8Path, Utf8PathBuf};
use shlex::Shlex;
use nust64::elf::Elf;
use nust64::rom::{Header, Rom};

//TODO:
// - insert file at specific location (extending ROM if necessary)

const LIBDRAGON_IPL3_PROD: &'static [u8] = include_bytes!("ipl3/ipl3_prod.z64");
const LIBDRAGON_IPL3_DEV: &'static [u8] = include_bytes!("ipl3/ipl3_dev.z64");
const LIBDRAGON_IPL3_COMPAT: &'static [u8] = include_bytes!("ipl3/ipl3_compat.z64");

#[derive(Debug, Clone, PartialEq, Bpaf)]
enum LibdragonIpl3Version {
    Compat,
    Debug,
    Release,
    Path(Utf8PathBuf),
}
impl FromStr for LibdragonIpl3Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "compat" => Self::Compat,
            "d" | "debug" | "dev" => Self::Debug,
            "r" | "release" | "prod" => Self::Release,
            s if Utf8PathBuf::from(s).is_file() => Self::Path(s.into()),
            _ => return Err("Unable to parse libdragon IPL3 version. Expected: compat, debug, or release".into()),
        })
    }
}

/// nust64 - ELF binary to N64 ROM converter
#[derive(Debug, Clone, Bpaf)]
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
    appends: Vec<Utf8PathBuf>,
    
    /// name to put in ROM header (max 20 bytes)
    #[bpaf(short, long)]
    name: Option<String>,
    
    /// Path to IPL3 binary. If omitted, libdragon's open-source IPL3 is used instead (https://github.com/rasky/libdragon/blob/ipl3/boot/README.md)
    #[bpaf(long)]
    ipl3: Option<Utf8PathBuf>,
    
    /// If '--ipl3' is not used, this determines which version of the libdragon open-source IPL3 is used. If omitted, the "prod" (release) version is used by default.
    /// 
    /// Valid options: compat, debug, release, or a filepath to a custom libdragon IPL3.
    #[bpaf(long)]
    libdragon: Option<LibdragonIpl3Version>,
    
    /// path to ELF file
    #[bpaf(long)]
    elf: Utf8PathBuf,
}

fn main() {
    let args = args().run();
    
    for pre in &args.pre_exec {
        exec(&pre);
    }
    
    let rom_path = args.elf.with_extension("z64");
    let rom = match args.ipl3.clone() {
        Some(path) => from_custom_ipl3(path, args.clone()),
        None => from_libdragon_ipl3(args.clone()),
    };
    
    fs::write(&rom_path, rom.to_vec()).unwrap();
    let rom_path = rom_path.canonicalize_utf8().unwrap_or(rom_path);
    println!("Generated ROM at: {rom_path}");
    
    for post in args.post_exec {
        exec(&post.replace(">>ROM<<", rom_path.to_string().as_str()));
    }
}

fn from_custom_ipl3<P: AsRef<Utf8Path>>(ipl3_path: P, args: Args) -> Rom {
    let ipl3_path = ipl3_path.as_ref();
    let elf_path = args.elf;
    
    let ipl3 = fs::read(ipl3_path).expect(&format!("IPL3 does not exist: {ipl3_path}"));
    if ipl3.len() < 4032 {
        println!("Warning! Provided IPL3 is smaller than 4032 bytes ({}). If this is unintentional, try padding the end of the file with zeros.", ipl3.len());
    }
    
    let elf = Elf::new(elf_path).expect("failed to parse ELF");
    
    let mut rom = Rom::new(&elf, &ipl3, args.name, args.sections);
    
    let data = &mut rom.binary;
    for append in args.appends {
        if append.is_file() {
            data.extend_from_slice(&fs::read(append).unwrap());
        }
    }
    
    rom
}

fn from_libdragon_ipl3(args: Args) -> Rom {
    let elf = Elf::new(&args.elf).expect("failed to parse ELF");
    
    use LibdragonIpl3Version::*;
    let build = args.libdragon.unwrap_or(Release);
    if build == Compat {
        let mut rom = Rom::new(&elf, &LIBDRAGON_IPL3_COMPAT[0x40..], args.name, args.sections);
        
        let data = &mut rom.binary;
        for append in args.appends {
            if append.is_file() {
                data.extend_from_slice(&fs::read(append).unwrap());
            }
        }
        
        rom
    } else {
        let libdragon = match build {
            Debug => LIBDRAGON_IPL3_DEV.to_vec(),
            Release => LIBDRAGON_IPL3_PROD.to_vec(),
            Path(path) => std::fs::read(path).expect("failed to read libdragon IPL3 file"),
            Compat => unreachable!(),
        };
        
        let mut binary = fs::read(&args.elf).unwrap();
        let mut header = Header::new(libdragon[..0x40].try_into().unwrap());
        header.pc = elf.entry;
        
        let mut name = args.name.unwrap_or_else(|| args.elf.file_name().unwrap().to_string()).as_bytes().to_vec();
        name.resize(20, ' ' as u8);
        header.image_name = name.try_into().unwrap();
        
        let misalignment = 256 - (libdragon.len() % 256);
        if misalignment > 0 {
            let mut aligned = vec![0x00; misalignment];
            aligned.extend_from_slice(&binary);
            
            binary = aligned;
        }
        
        Rom {
            header,
            ipl3: libdragon[0x40..].to_vec(),
            binary,
        }
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