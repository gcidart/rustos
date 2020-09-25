use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;

use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Path;

use crate::mbr::MasterBootRecord;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::SliceExt;
use crate::vfat::{BiosParameterBlock, CachedPartition, Partition};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Status};
use std::convert::TryFrom;

/// A generic trait that handles a critical section as a closure
pub trait VFatHandle: Clone + Debug + Send + Sync {
    fn new(val: VFat<Self>) -> Self;
    fn lock<R>(&self, f: impl FnOnce(&mut VFat<Self>) -> R) -> R;
}

#[derive(Debug)]
pub struct VFat<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    device: CachedPartition,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    rootdir_cluster: Cluster,
}

impl<HANDLE: VFatHandle> VFat<HANDLE> {
    pub fn from<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
        let mbr = MasterBootRecord::from(&mut device)?;
        let mut start: u64 = mbr.partition_table_entry[0].relative_sector.into();
        for i in 0..4 {
            if mbr.partition_table_entry[i].partition_type == 0xB || mbr.partition_table_entry[i].partition_type == 0xC {
                start = mbr.partition_table_entry[i].relative_sector.into();
                break;
            }
        }
        let bpb = BiosParameterBlock::from(&mut device, start)?;
        let par = Partition {
            start : start,
            num_sectors : bpb.logical_sectors.into(),
            sector_size : bpb.bytes_per_sector.into()
        };
        let reserved_sectors: u64 = bpb.reserved_sectors.into();
        let sectors_per_fat: u64 = bpb.sectors_per_fat.into();
        let num_fat: u64 = bpb.num_fat.into();
        let vfat = VFat {
            phantom : PhantomData,
            device : CachedPartition::new(device, par),
            bytes_per_sector : bpb.bytes_per_sector,
            sectors_per_cluster : bpb.sectors_per_cluster,
            sectors_per_fat : bpb.sectors_per_fat,
            fat_start_sector : reserved_sectors ,
            data_start_sector : reserved_sectors  + sectors_per_fat * num_fat,
            rootdir_cluster : Cluster::from(bpb.root_dir_cluster_num)
        };
        return Ok(VFatHandle::new(vfat));
    }

    //  * A method to read from an offset of a cluster into a buffer.
    
        fn read_cluster(
            &mut self,
            cluster: Cluster,
            offset: usize,
            buf: &mut [u8]
        ) -> io::Result<usize> 
        {
            let mut start_index = 0;
            let mut sector = self.data_start_sector + (cluster.cluster_num()-2)*(self.sectors_per_cluster as u64);
            for _ in 0..self.sectors_per_cluster {
                let slice_index = start_index..(start_index+self.bytes_per_sector as usize);
                self.device.read_sector(sector, &mut buf[slice_index])?;
                sector+= 1;
                start_index+=self.bytes_per_sector as usize;
            }
            return Ok(start_index);
                
        }
    
    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
   
        pub fn read_chain(
            &mut self,
            start: Cluster,
            buf: &mut Vec<u8>
        ) -> io::Result<usize>
        {
            let mut read_size = 0;
            let mut start_copy = start;
            let sector_size = self.bytes_per_sector as usize;
            loop {
                let vstart = buf.len();
                buf.resize(vstart + sector_size*(self.sectors_per_cluster as usize), 0);
                let slice_index = vstart..(vstart + sector_size*(self.sectors_per_cluster as usize));
                if let Some(sbuf) = buf.get_mut(slice_index) {
                    read_size += self.read_cluster(start_copy, 0, sbuf)?;
                }  else {
                    return Err(io::Error::new(io::ErrorKind::Other,"get_mut failed"));
                }
                match self.fat_entry(start_copy)?.status() {
                    Status::Data(c) => start_copy = c,
                    Status::Eoc(_) => break,
                    _ => return Err(io::Error::new(io::ErrorKind::Other, "Invalid Fat Entry"))
                };
            }
            Ok(read_size)
        }

    //
    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    //
        fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry>
        {
            let fat_entries_per_sector = (self.bytes_per_sector as u64)/4;
            let sector_num = self.fat_start_sector + (cluster.cluster_num() as u64)/(fat_entries_per_sector);
            let sector_offset = (((cluster.cluster_num() as u64) % (fat_entries_per_sector)) as usize)*4;
            let sector_ref = self.device.get(sector_num)?.get(sector_offset..(sector_offset+4)).unwrap();
            let temp: &[FatEntry] = unsafe {
                sector_ref.cast()
            };
            return Ok(&temp[0]);
        }
    //
    // A method to return Root Directory Cluster
    //
        pub fn rootdir_cluster(& self) -> Cluster
        {
            return self.rootdir_cluster;
        }
}

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = File<HANDLE>;
    type Dir =  Dir<HANDLE>;
    type Entry = Entry<HANDLE>;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        let components: Vec<_> = path.as_ref().components().map(|comp| comp.as_os_str()).collect();
        let rootdir_cluster = self.lock(|vfat_instance| {vfat_instance.rootdir_cluster()});
        let mut dir = Dir {
            vfat : self.clone(),
            first_cluster : Cluster::from(rootdir_cluster),
            file_name : String::from("/"),
            metadata : Default::default(),
        };
        if components.len()==1 {
            return Ok(Entry::DIR(dir));
        } else {
            for i in 1..components.len()-1 {
                let entry = dir.find(components[i])?;
                match entry {
                    Entry::DIR(d) => dir =d,
                    _ => return Err(io::Error::new(io::ErrorKind::Other, "Unexpected Path Component")),
                };
            }
            return dir.find(components[components.len()-1]);
        }
    }
}
