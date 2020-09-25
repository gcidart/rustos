use alloc::string::String;

use shim::io::{self, SeekFrom};

use crate::traits;
use crate::vfat::{Cluster, Metadata, VFatHandle,VFat};

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub first_cluster: Cluster,
    pub file_name : String,
    pub metadata : Metadata,
    pub file_size : u64,
    pub file_offset : usize
}

/// `traits::File` (and its supertraits) for `File`.

impl<HANDLE:VFatHandle> traits::File for File<HANDLE> {
    /// Writes any buffered data to disk.
    fn sync(&mut self) -> io::Result<()> {
        return Ok(());
    }

    /// Returns the size of the file in bytes.
    fn size(&self) -> u64 {
        return self.file_size;
    }
}

impl<HANDLE: VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        panic!("Dummy")
    }
    fn flush(&mut self) -> io::Result<()> {
        panic!("Dummy")
    }
}

impl<HANDLE:VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.file_size==0 {
            return Ok(0);
        }
        let mut vec : Vec<u8> = Vec::new();
        self.vfat.lock(|vfat_instance| {vfat_instance.read_chain(self.first_cluster,&mut vec).unwrap()});
        let start = self.file_offset;
        let end = std::cmp::min(self.file_offset + buf.len(), self.file_size as usize);
        if start==end {
            return Ok(0);
        }
        for i in start..end {
            buf[i-start] = vec[i];
        }
        self.file_offset = end;
        return Ok(end-start);
    }
}

impl<HANDLE: VFatHandle> io::Seek for File<HANDLE> {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        match _pos {
            SeekFrom::Start(s) => {
                if s< self.file_size {
                    return Ok(s);
                } else {
                    return  Err(io::Error::new(io::ErrorKind::InvalidInput,"seek beyond end "));
                }
            },
            _ =>  return  Err(io::Error::new(io::ErrorKind::InvalidInput,"seek beyond end ")),
        }

    }
}
