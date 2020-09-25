use core::fmt;
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    pub head: u8,
    pub sector: u8,
    pub cylinder: u8,
}

impl fmt::Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let head: u8 = {self.head};
        let sector:u16 = {self.sector} as u16;
        let cylinder:u16 = {self.cylinder} as u16;
        let cylinder:u16 = (cylinder << 2) | ((sector & 0b1100_0000)>>6);
        let sector:u16 = sector & 0b11_1111;
        f.debug_struct("CHS")
            .field("head", &head)
            .field("sector", &sector)
            .field("cylinder", &cylinder)
            .finish()
    }
}

const_assert_size!(CHS, 3);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct PartitionEntry {
    pub boot_indicator: u8,
    pub starting_chs: CHS,
    pub partition_type: u8,
    pub ending_chs: CHS,
    pub relative_sector: u32,
    pub total_sectors: u32,

}

impl fmt::Debug for PartitionEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rs_temp = &{self.relative_sector};
        let ts_temp = &{self.total_sectors};
        f.debug_struct("PartitionEntry")
            .field("boot indicator", &self.boot_indicator)
            .field("starting head, sector, cylinder", &self.starting_chs)
            .field("partition type", &self.partition_type)
            .field("ending head, sector, cylinder", &self.ending_chs)
            .field("relative sector", rs_temp)
            .field("total sectors", ts_temp)
            .finish()
    }
}


const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    pub bootstrap: [u8; 436],
    pub disk_id: [u8; 10],
    pub partition_table_entry: [PartitionEntry; 4],
    pub signature_bytes: [u8; 2],
}

impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MasterBootRecord")
            .field("disk ID", &self.disk_id)
            .field("partition_table_entry", &self.partition_table_entry)
            .field("signature bytes", &self.signature_bytes)
            .finish()
    }
}

const_assert_size!(MasterBootRecord, 512);

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        let mut buf : [u8; 512] = [0; 512];
        match device.read_sector(0, &mut buf) {
            Err(e) => Err(Error::Io(e)),
            Ok(_) => {
                let mut bootstrap: [u8; 436] = [0; 436];
                let mut disk_id: [u8; 10] = [0; 10];
                let chs = CHS {
                    head: 0,
                    sector: 0,
                    cylinder: 0
                };
                let partition_entry = PartitionEntry {
                    boot_indicator: 0,
                    starting_chs: chs,
                    partition_type: 0,
                    ending_chs: chs,
                    relative_sector: 0,
                    total_sectors: 0,
                };
                let mut partition_table_entry : [PartitionEntry; 4] = [partition_entry; 4];
                let mut signature_bytes: [u8; 2] = [0; 2];
                for i in 0..436 {
                    bootstrap[i] = buf[i];
                }
                for i in 436..446 {
                    disk_id[i-436] = buf[i];
                }
                let mut mbr_offset = 446;
                for i in 0..4 {
                    if buf[mbr_offset] != 0 && buf[mbr_offset]!=0x80 {
                        return Err(Error::UnknownBootIndicator(i as u8));
                    }
                    let idx = i as usize;
                    partition_table_entry[idx].boot_indicator = buf[mbr_offset];
                    mbr_offset += 1;
                    partition_table_entry[idx].starting_chs.head = buf[mbr_offset];
                    mbr_offset += 1;
                    partition_table_entry[idx].starting_chs.sector = buf[mbr_offset];
                    mbr_offset += 1;
                    partition_table_entry[idx].starting_chs.cylinder = buf[mbr_offset];
                    mbr_offset += 1;
                    partition_table_entry[idx].partition_type = buf[mbr_offset];
                    mbr_offset += 1;
                    partition_table_entry[idx].ending_chs.head = buf[mbr_offset];
                    mbr_offset += 1;
                    partition_table_entry[idx].ending_chs.sector = buf[mbr_offset];
                    mbr_offset += 1;
                    partition_table_entry[idx].ending_chs.cylinder = buf[mbr_offset];
                    mbr_offset += 1;
                    let mut temp : [u8; 4] = [0; 4];
                    temp[0] = buf[mbr_offset];
                    temp[1] = buf[mbr_offset+1];
                    temp[2] = buf[mbr_offset+2];
                    temp[3] = buf[mbr_offset+3];
                    partition_table_entry[idx].relative_sector = u32::from_le_bytes(temp);
                    mbr_offset += 4;
                    temp[0] = buf[mbr_offset];
                    temp[1] = buf[mbr_offset+1];
                    temp[2] = buf[mbr_offset+2];
                    temp[3] = buf[mbr_offset+3];
                    partition_table_entry[idx].total_sectors = u32::from_le_bytes(temp);
                    mbr_offset += 4;
                }
                signature_bytes[0] = buf[mbr_offset];
                mbr_offset += 1;
                signature_bytes[1] = buf[mbr_offset];
                if signature_bytes[0] != 0x55 && signature_bytes[1] != 0xAA {
                    return Err(Error::BadSignature);
                } else {
                    return Ok(MasterBootRecord{
                        bootstrap: bootstrap,
                        disk_id: disk_id,
                        partition_table_entry: partition_table_entry,
                        signature_bytes: signature_bytes});
                }
            },
        }
    }

}
