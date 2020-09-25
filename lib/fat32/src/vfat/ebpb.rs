use core::fmt;
use shim::const_assert_size;

use crate::traits::BlockDevice;
use crate::vfat::Error;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    pub jmp_bytes: [u8; 3],
    pub oem_id: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fat: u8,
    pub max_num_dir_entries: u16,
    pub total_logical_sectors: u16,
    pub media_descriptor_type: u8,
    pub sectors_per_fat_u16: u16,
    pub sectors_per_track: u16,
    pub num_heads: u16,
    pub hidden_sectors: u32,
    pub logical_sectors:u32,
    pub sectors_per_fat: u32,
    pub flags: u16,
    pub fat_ver_num: [u8; 2],
    pub root_dir_cluster_num: u32,
    pub fsinfo_sector_num: u16,
    pub bkp_boot_sector_num: u16,
    pub reserved: [u8; 12],
    pub drive_num: u8,
    pub win_flag: u8,
    pub signature: u8,
    pub volume_id_sno: [u8; 4],
    pub volume_label: [u8; 11],
    pub system_id: [u8; 8],
    pub boot_code: [u8; 420],
    pub bootable_partition_signature: [u8; 2],

}

const_assert_size!(BiosParameterBlock, 512);

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut buf : [u8; 512] = [0; 512];
        match device.read_sector(sector, &mut buf) {
            Err(e) => Err(Error::Io(e)),
            Ok(_) => {
                let mut bpb = BiosParameterBlock {
                    jmp_bytes: [0; 3],
                    oem_id: [0; 8],
                    bytes_per_sector: 0,
                    sectors_per_cluster: 0,
                    reserved_sectors: 0,
                    num_fat: 0,
                    max_num_dir_entries: 0,
                    total_logical_sectors: 0,
                    media_descriptor_type: 0,
                    sectors_per_fat_u16: 0,
                    sectors_per_track: 0,
                    num_heads: 0,
                    hidden_sectors: 0,
                    logical_sectors:0,
                    sectors_per_fat: 0,
                    flags: 0,
                    fat_ver_num: [0; 2],
                    root_dir_cluster_num: 0,
                    fsinfo_sector_num: 0,
                    bkp_boot_sector_num: 0,
                    reserved: [0; 12],
                    drive_num: 0,
                    win_flag: 0,
                    signature: 0,
                    volume_id_sno: [0; 4],
                    volume_label: [0; 11],
                    system_id: [0; 8],
                    boot_code: [0; 420],
                    bootable_partition_signature: [0; 2],
                }; 
                let mut bpb_offset = 0;
                let mut temp16 : [u8; 2] = [0; 2];
                let mut temp32 : [u8; 4] = [0; 4];
                for i in 0..3 {
                    bpb.jmp_bytes[i] = buf[bpb_offset+i];
                }
                bpb_offset += 3;
                for i in 0..8 {
                    bpb.oem_id[i] = buf[bpb_offset+i];
                }
                bpb_offset += 8;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.bytes_per_sector = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                bpb.sectors_per_cluster = buf[bpb_offset];
                bpb_offset += 1;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.reserved_sectors = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                bpb.num_fat = buf[bpb_offset];
                bpb_offset += 1;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.max_num_dir_entries = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.total_logical_sectors = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                bpb.media_descriptor_type = buf[bpb_offset];
                bpb_offset += 1;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.sectors_per_fat_u16 = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.sectors_per_track = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.num_heads = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                for i in 0..4 {
                    temp32[i] = buf[bpb_offset+i];
                }
                bpb.hidden_sectors = u32::from_le_bytes(temp32);
                bpb_offset += 4;
                for i in 0..4 {
                    temp32[i] = buf[bpb_offset+i];
                }
                bpb.logical_sectors = u32::from_le_bytes(temp32);
                bpb_offset += 4;
                for i in 0..4 {
                    temp32[i] = buf[bpb_offset+i];
                }
                bpb.sectors_per_fat = u32::from_le_bytes(temp32);
                bpb_offset += 4;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.flags = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                for i in 0..2 {
                    bpb.fat_ver_num[i] = buf[bpb_offset+i];
                }
                bpb_offset += 2;
                for i in 0..4 {
                    temp32[i] = buf[bpb_offset+i];
                }
                bpb.root_dir_cluster_num = u32::from_le_bytes(temp32);
                bpb_offset += 4;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.fsinfo_sector_num = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                for i in 0..2 {
                    temp16[i] = buf[bpb_offset+i];
                }
                bpb.bkp_boot_sector_num = u16::from_le_bytes(temp16);
                bpb_offset += 2;
                for i in 0..12 {
                    bpb.reserved[i] = buf[bpb_offset+i];
                }
                bpb_offset += 12;
                bpb.drive_num = buf[bpb_offset];
                bpb_offset += 1;
                bpb.win_flag = buf[bpb_offset];
                bpb_offset += 1;
                bpb.signature = buf[bpb_offset];
                bpb_offset += 1;
                for i in 0..4 {
                    bpb.volume_id_sno[i] = buf[bpb_offset+i];
                }
                bpb_offset += 4;
                for i in 0..11 {
                    bpb.volume_label[i] = buf[bpb_offset+i];
                }
                bpb_offset += 11;
                for i in 0..8 {
                    bpb.system_id[i] = buf[bpb_offset+i];
                }
                bpb_offset += 8;
                for i in 0..420 {
                    bpb.boot_code[i] = buf[bpb_offset+i];
                }
                bpb_offset += 420;
                for i in 0..2 {
                    bpb.bootable_partition_signature[i] = buf[bpb_offset+i];
                }
                if bpb.bootable_partition_signature[0] != 0x55 && bpb.bootable_partition_signature[1] != 0xAA {
                    return Err(Error::BadSignature);
                } else {
                    return Ok(bpb);
                }
            }
        }

    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BiosParameterBlock")
            .field("jmp bytes", &self.jmp_bytes)
            .field("oem_id", &self.oem_id)
            .field("bytes_per_sector", &{self.bytes_per_sector})
            .field("sectors_per_cluster", &self.sectors_per_cluster)
            .field("reserved_sectors", &{self.reserved_sectors})
            .field("num_fat", &self.num_fat)
            .field("max_num_dir_entries", &{self.max_num_dir_entries})
            .field("total_logical_sectors", &{self.total_logical_sectors})
            .field("media_descriptor_type", &self.media_descriptor_type)
            .field("sectors_per_fat_u16", &{self.sectors_per_fat_u16})
            .field("sectors_per_track", &{self.sectors_per_track})
            .field("num_heads", &{self.num_heads})
            .field("hidden_sectors", &{self.hidden_sectors})
            .field("logical_sectors", &{self.logical_sectors})
            .field("sectors_per_fat", &{self.sectors_per_fat})
            .field("flags", &{self.flags})
            .field("fat_ver_num", &self.fat_ver_num)
            .field("root_dir_cluster_num", &{self.root_dir_cluster_num})
            .field("fsinfo_sector_num", &{self.fsinfo_sector_num})
            .field("bkp_boot_sector_num", &{self.bkp_boot_sector_num})
            .field("reserved", &self.reserved)
            .field("drive_num", &self.drive_num)
            .field("win_flag", &self.win_flag)
            .field("signature", &self.signature)
            .field("volume_id_sno", &self.volume_id_sno)
            .field("volume_label", &self.volume_label)
            .field("system_id", &self.system_id)
            .field("bootable_partition_signature", &self.bootable_partition_signature)
            .finish()
    }
}
