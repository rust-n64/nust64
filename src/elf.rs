use std::path::{Path, PathBuf};
use object::{File, Object, ObjectSection, SectionFlags, SectionKind};
use object::elf::SHF_EXECINSTR;
use crate::Result;

/// Simplified version of an ELF object section.
#[derive(Clone, PartialEq, Debug)]
pub struct ElfSection {
    pub name: Option<String>,
    pub addr: u64,
    pub data: Vec<u8>,
    pub flags: u64,
    pub kind: SectionKind,
}

/// Result of parsing an ELF object file, this stores the important components for generating 
/// a [Rom](crate::rom::Rom).
#[derive(Clone, PartialEq, Debug)]
pub struct Elf {
    pub path: PathBuf,
    pub raw: Vec<u8>,
    pub entry: u32,
    pub sections: Vec<ElfSection>,
}
impl Elf {
    /// Loads an ELF object file, and parses the most critical information from it for use with
    /// this crate. Additional ELF data can be retrieved using [`Self::object()`].
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        match std::fs::read(path.as_ref()) {
            Ok(raw) => {
                let obj = File::parse(raw.as_slice())?;
                let entry = obj.entry() as u32;
                
                let mut sections = vec![];
                for section in obj.sections() {
                    sections.push(ElfSection {
                        name: section.name().ok().map(|name| name.to_string()),
                        addr: section.address(),
                        data: section.data().unwrap_or_default().to_vec(),
                        flags: match section.flags() {
                            SectionFlags::Elf { sh_flags } => sh_flags,
                            _ => 0
                        },
                        kind: section.kind(),
                    });
                }
                sections.sort_by(|a, b| a.addr.cmp(&b.addr));
                
                Ok(Self {
                    path: path.as_ref().to_path_buf(),
                    raw,
                    entry,
                    sections,
                })
            },
            Err(err) => Err(err.into())
        }
    }
    
    pub fn object(&self) -> object::Result<File> {
        File::parse(self.raw.as_slice())
    }
    
    pub fn section_by_name<S: ToString>(&self, name: S) -> Option<&ElfSection> {
        self.sections.iter().find(|section| section.name == Some(name.to_string()))
    }
    
    pub fn is_executable(&self) -> bool {
        match self.section_by_name(".boot") {
            Some(section) => (section.flags & (SHF_EXECINSTR as u64)) != 0,
            _ => false,
        }
    }
}