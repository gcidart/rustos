use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use hashbrown::HashMap;
use shim::io;

use crate::traits::BlockDevice;

#[derive(Debug)]
struct CacheEntry {
    data: Vec<u8>,
    dirty: bool,
}

pub struct Partition {
    /// The physical sector where the partition begins.
    pub start: u64,
    /// Number of sectors
    pub num_sectors: u64,
    /// The size, in bytes, of a logical sector in the partition.
    pub sector_size: u64,
}

pub struct CachedPartition {
    device: Box<dyn BlockDevice>,
    cache: HashMap<u64, CacheEntry>,
    partition: Partition,
}

impl CachedPartition {
    /// Creates a new `CachedPartition` that transparently caches sectors from
    /// `device` and maps physical sectors to logical sectors inside of
    /// `partition`. All reads and writes from `CacheDevice` are performed on
    /// in-memory caches.
    ///
    /// The `partition` parameter determines the size of a logical sector and
    /// where logical sectors begin. An access to a sector `0` will be
    /// translated to physical sector `partition.start`. Virtual sectors of
    /// sector number `[0, num_sectors)` are accessible.
    ///
    /// `partition.sector_size` must be an integer multiple of
    /// `device.sector_size()`.
    ///
    /// # Panics
    ///
    /// Panics if the partition's sector size is < the device's sector size.
    pub fn new<T>(device: T, partition: Partition) -> CachedPartition
    where
        T: BlockDevice + 'static,
    {
        assert!(partition.sector_size >= device.sector_size());

        CachedPartition {
            device: Box::new(device),
            cache: HashMap::new(),
            partition: partition,
        }
    }

    /// Returns the number of physical sectors that corresponds to
    /// one logical sector.
    fn factor(&self) -> u64 {
        self.partition.sector_size / self.device.sector_size()
    }

    /// Maps a user's request for a sector `virt` to the physical sector.
    /// Returns `None` if the virtual sector number is out of range.
    fn virtual_to_physical(&self, virt: u64) -> Option<u64> {
        if virt >= self.partition.num_sectors {
            return None;
        }

        let physical_offset = virt * self.factor();
        let physical_sector = self.partition.start + physical_offset;

        Some(physical_sector)
    }

    /// Returns a mutable reference to the cached sector `sector`. If the sector
    /// is not already cached, the sector is first read from the disk.
    ///
    /// The sector is marked dirty as a result of calling this method as it is
    /// presumed that the sector will be written to. If this is not intended,
    /// use `get()` instead.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get_mut(&mut self, sector: u64) -> io::Result<&mut [u8]> {
        let factor: usize = self.factor() as usize;
        let mut physical_sector: u64 = 0;
        if let Some(ps) = self.virtual_to_physical(sector) {
            physical_sector = ps;
        }
        else  {
            return Err(io::Error::new(io::ErrorKind::Other,"virtual sector out of range"));
        }
        if self.cache.contains_key(&sector) == false {
            let mut vec = Vec::with_capacity(self.partition.sector_size as usize);
            vec.resize(self.partition.sector_size as usize, 0);
            for i in 0..factor {
                let slice_start = i*(self.device.sector_size() as usize);
                let slice_end = (i+1)*(self.device.sector_size() as usize);
                if let Some(buf) = vec.get_mut(slice_start..slice_end) {
                    self.device.read_sector(physical_sector + (i as u64), buf)?;
                }
            }
            let tce = CacheEntry {
                data : vec,
                dirty : false
            };
            self.cache.insert(sector, tce);
        }
        match self.cache.get_mut(&sector) {
            Some(ce) => {
                if ce.dirty == false {
                    if let Some(buf) = ce.data.get_mut(0..(self.partition.sector_size as usize)) {
                        return Ok(buf);
                    } else {
                        ce.dirty = true;
                        return Err(io::Error::new(io::ErrorKind::Other,"get_mut failed"));
                    }
                } else {
                    let mut vec = Vec::with_capacity(self.partition.sector_size as usize);
                    vec.resize(self.partition.sector_size as usize, 0);
                    for i in 0..factor  {
                        let slice_start = i*(self.device.sector_size() as usize);
                        let slice_end = (i+1)*(self.device.sector_size() as usize);
                        if let Some(buf) = vec.get_mut(slice_start..slice_end) {
                            self.device.read_sector(physical_sector + (i as u64), buf)?;
                        }
                    }
                    ce.data = vec;
                    ce.dirty = true;
                    if let Some(buf) = ce.data.get_mut(0..(self.partition.sector_size as usize)) {
                        return Ok(buf);
                    } else {
                        return Err(io::Error::new(io::ErrorKind::Other,"get_mut failed"));
                    }
                }
            },
            None => {
                return Err(io::Error::new(io::ErrorKind::Other,"Sector not found in cache"));
            }
        }
    }

    /// Returns a reference to the cached sector `sector`. If the sector is not
    /// already cached, the sector is first read from the disk.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get(&mut self, sector: u64) -> io::Result<&[u8]> {
        let factor: usize = self.factor() as usize;
        let mut physical_sector: u64 = 0;
        if let Some(ps) = self.virtual_to_physical(sector) {
            physical_sector = ps;
        }
        else  {
            return Err(io::Error::new(io::ErrorKind::Other,"virtual sector out of range"));
        }
        if self.cache.contains_key(&sector) == false {
            let mut vec = Vec::with_capacity(self.partition.sector_size as usize);
            vec.resize(self.partition.sector_size as usize, 0);
            for i in 0..factor {
                let slice_start = i*(self.device.sector_size() as usize);
                let slice_end = (i+1)*(self.device.sector_size() as usize);
                if let Some(buf) = vec.get_mut(slice_start..slice_end) {
                    self.device.read_sector(physical_sector + (i as u64), buf)?;
                }
            }
            let tce = CacheEntry {
                data : vec,
                dirty : false
            };
            self.cache.insert(sector, tce);
        }
        match self.cache.get_mut(&sector) {
            Some(ce) => {
                if ce.dirty == false {
                    if let Some(buf) = ce.data.get(0..(self.partition.sector_size as usize)) {
                        return Ok(buf);
                    } else {
                        return Err(io::Error::new(io::ErrorKind::Other,"get failed"));
                    }
                } else {
                    let mut vec = Vec::with_capacity(self.partition.sector_size as usize);
                    vec.resize(self.partition.sector_size as usize, 0);
                    for i in 0..factor  {
                        let slice_start = i*(self.device.sector_size() as usize);
                        let slice_end = (i+1)*(self.device.sector_size() as usize);
                        if let Some(buf) = vec.get_mut(slice_start..slice_end) {
                            self.device.read_sector(physical_sector + (i as u64), buf)?;
                        }
                    }
                    ce.data = vec;
                    ce.dirty = true;
                    if let Some(buf) = ce.data.get(0..(self.partition.sector_size as usize)) {
                        return Ok(buf);
                    } else {
                        return Err(io::Error::new(io::ErrorKind::Other,"get failed"));
                    }
                }
            },
            None => {
                return Err(io::Error::new(io::ErrorKind::Other,"Sector not found in cache"));
            }
        }
    }
}

/// `BlockDevice` for `CacheDevice`. The `read_sector` and
/// `write_sector` methods only reads/writes from/to cached sectors.
impl BlockDevice for CachedPartition {
    fn sector_size(&self) -> u64 {
        self.partition.sector_size
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> io::Result<usize> {
        let sector_size: usize= self.partition.sector_size as usize;
        match self.get(sector) {
            Ok(s) => {
                for i in 0..sector_size {
                    buf[i] = s[i];
                }
                Ok(sector_size)
            },
            Err(e) => Err(e)
        }
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> io::Result<usize> {
        let sector_size: usize= self.partition.sector_size as usize;
        match self.get_mut(sector) {
            Ok(s) => {
                for i in 0..sector_size {
                    s[i] = buf[i];
                }
                Ok(sector_size)
            },
            Err(e) => Err(e)
        }
    }
}

impl fmt::Debug for CachedPartition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CachedPartition")
            .field("device", &"<block device>")
            .field("cache", &self.cache)
            .finish()
    }
}
