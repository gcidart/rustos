use alloc::string::String;
use alloc::vec::Vec;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;

use crate::traits;
use crate::util::VecExt;
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle};
#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub first_cluster: Cluster,
    pub file_name : String,
    pub metadata : Metadata
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    pub file_name : [u8; 8],
    pub file_ext : [u8; 3],
    pub attributes : Attributes,
    pub reserved : u8,
    pub creation_time_tenth_sec : u8,
    pub creation_time : Time,
    pub creation_date : Date,
    pub accessed_date : Date,
    pub first_cluster_high : u16,
    pub modified_time : Time,
    pub modified_date : Date,
    pub first_cluster_low : u16,
    pub file_size : u32
}

const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    pub seq_no : u8,
    pub name_char_1 : [u16; 5],
    pub attributes : Attributes,
    pub lfn_type : u8,
    pub cksum_file_name : u8,
    pub name_char_2 : [u16; 6],
    pub zero : u16,
    pub name_char_3 : [u16; 2]
}

const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    pub entry :[u8; 32]
}

const_assert_size!(VFatUnknownDirEntry, 32);

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry<HANDLE>> {
        use traits::Dir;
        let mut ei = self.entries()?;
        loop{
            match ei.next(){
                Some(e) => {
                    match &e {
                        Entry::DIR(d) => {
                            if let Some(ps) = name.as_ref().to_str() {
                                if ps.eq_ignore_ascii_case(&d.file_name) {
                                    return Ok(e);
                                }
                            } else {
                                return Err(newioerr!(InvalidInput, "invalid UTF-8"));
                            }
                        },
                        Entry::FILE(f) => {
                            if let Some(ps) = name.as_ref().to_str() {
                                if ps.eq_ignore_ascii_case(&f.file_name) {
                                    return Ok(e);
                                }
                            } else {
                                return Err(newioerr!(InvalidInput, "invalid UTF-8"));
                            }
                        },
                        _ => continue,
                    };
                },
                None => break,
            }
        }
        return Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
    }
}

pub struct EntryIterator<HANDLE:VFatHandle> {
    buf : Vec<u8>,
    offset: usize,
    vfat:HANDLE
}
impl<HANDLE: VFatHandle> EntryIterator<HANDLE> {
    fn new(vfat: HANDLE) -> EntryIterator<HANDLE> {
        EntryIterator {
            buf : Vec::new(),
            offset : 0,
            vfat : vfat.clone(),
        }
    }
}

impl<HANDLE: VFatHandle> Iterator for EntryIterator<HANDLE> {
    type Item = Entry<HANDLE>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut filename = String::new();
        let mut filename_vec : Vec<String> = Vec::new();
        filename_vec.resize(32, String::new());
        loop {
            let mut vbuf : [u8; 32] = [0; 32];
            for i in 0..32 {
                vbuf[i] = self.buf[self.offset+i];
            }
            let vfde = VFatDirEntry { unknown : VFatUnknownDirEntry{entry : vbuf} };
            let offset_eleven = unsafe {vfde.unknown.entry[11]};
            let is_file = offset_eleven & 0x10 ==0;
            if offset_eleven == 0x0f {
                let vflde = unsafe {vfde.long_filename};
                if vflde.seq_no == 0xe5 || vflde.seq_no == 0 {
                    self.offset+=32;
                    continue;
                }
                let mut vec = Vec::new();
                for i in 0..5{
                    if vflde.name_char_1[i]!= 0x0000 && vflde.name_char_1[i]!=0xffff {
                        vec.push(vflde.name_char_1[i]);
                    }
                }
                for i in 0..6{
                    if vflde.name_char_2[i]!= 0x0000 && vflde.name_char_2[i]!=0xffff {
                        vec.push(vflde.name_char_2[i]);
                    }
                }
                for i in 0..2{
                    if vflde.name_char_3[i]!= 0x0000 && vflde.name_char_3[i]!=0xffff {
                        vec.push(vflde.name_char_3[i]);
                    }
                }
                filename_vec[(vflde.seq_no & 0x1f) as usize] = String::from_utf16(&vec).unwrap();
                self.offset += 32;
            } else {
                let vfrde = unsafe {vfde.regular};
                let mut fnv = vfrde.file_name.to_vec();
                let mut filename_size  = 8;
                if fnv[0] == 0xe5 {
                    self.offset+= 32;
                    continue;
                }
                let mut filename_vec_size = 32;
                for i in (0..32).rev() {
                    if filename_vec[i] == "" {
                        filename_vec_size-= 1;
                    } else {
                        break;
                    }
                }
                filename_vec.resize(filename_vec_size, String::new());
                if fnv[0] == 0x0 {
                    return None;
                }
                if filename_vec.len() > 0 {
                    for i in 0..filename_vec.len() {
                        filename.push_str(&filename_vec[i]);
                    }
                } else {
                    for i in (0..8).rev() {
                        if fnv[i]!=0 && fnv[i]!=0x20 {
                            break;
                        }
                        filename_size-= 1;
                    }
                    fnv.resize(filename_size, 0);
                    if filename_size > 0 {
                        let regular_filename = String::from_utf8(fnv).unwrap();
                        filename.push_str(&regular_filename);
                    }
                    let mut fev = vfrde.file_ext.to_vec();
                    let mut fileext_size  = 3;
                    for i in (0..3).rev() {
                        if fev[i]!=0 && fev[i]!=0x20 {
                            break;
                        }
                        fileext_size-= 1;
                    }
                    fev.resize(fileext_size, 0);
                    if fileext_size > 0 {
                        filename.push_str(".");
                        let regular_extname = String::from_utf8(fev).unwrap();
                        filename.push_str(&regular_extname);
                    }
                }
                let fcn: u32 = (vfrde.first_cluster_high as u32)<<16 | (vfrde.first_cluster_low as u32);
                let metadata = Metadata {
                    attributes : vfrde.attributes,
                    created : Timestamp {
                            date : vfrde.creation_date,
                            time : vfrde.creation_time,
                    },
                    accessed : Timestamp {
                            date : vfrde.accessed_date,
                            time : Default::default(),
                    },
                    modified : Timestamp {
                            date : vfrde.modified_date,
                            time : vfrde.modified_time,
                    },
                };
                self.offset += 32;
                if is_file {
                    let nfile = File {
                        vfat: self.vfat.clone(),
                        first_cluster : Cluster::from(fcn),
                        file_name : filename,
                        metadata : metadata,
                        file_size: vfrde.file_size as u64,
                        file_offset: 0,
                    };
                    return Some(Entry::FILE(nfile));
                } else {
                    let ndir = Dir {
                        vfat : self.vfat.clone(),
                        first_cluster : Cluster::from(fcn),
                        file_name : filename,
                        metadata : metadata,
                    };
                    return Some(Entry::DIR(ndir));
                }
            }
        }
    }

}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    /// The type of entry stored in this directory.
    type Entry = Entry<HANDLE>;

    /// An type that is an iterator over the entries in this directory.
    type Iter= EntryIterator<HANDLE>;

    /// Returns an iterator over the entries in this directory.
    fn entries(&self) -> io::Result<Self::Iter >{
        let mut ei = EntryIterator::new(self.vfat.clone());
        self.vfat.lock(|vfat_instance| {vfat_instance.read_chain(self.first_cluster, &mut ei.buf)})?;
        return Ok(ei);
    }

}
