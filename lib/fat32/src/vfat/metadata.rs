use core::fmt;

use alloc::string::String;

use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    pub attributes : Attributes,
    pub created : Timestamp,
    pub accessed : Timestamp,
    pub modified : Timestamp,
}

impl traits::Timestamp for Timestamp {
    fn year(&self) ->usize {
        let mut year : usize = 1980;
        year += ((self.date.0 & 0b1111111000000000) >> 9 ) as usize;
        return year;
    }

    fn month(&self) ->u8 {
        let mut month : u8 = 0;
        month += ((self.date.0 & 0b111100000) >> 5) as u8;
        return month;
    }

    fn day(&self) ->u8 {
        let mut day : u8 = 0;
        day += (self.date.0 & 0b11111)  as u8;
        return day;
    }

    fn hour(&self) ->u8 {
        let mut hour : u8 = 0;
        hour += ((self.time.0 & 0b1111100000000000) >> 11)  as u8;
        return hour;
    }

    fn minute(&self) ->u8 {
        let mut min : u8 = 0;
        min += ((self.time.0 & 0b11111100000) >> 5)  as u8;
        return min;
    }

    fn second(&self) ->u8 {
        let mut sec : u8 = 0;
        sec += (self.time.0 & 0b11111)   as u8;
        sec *= 2;
        return sec;
    }
}
impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;
    /// Whether the associated entry is read only.
    fn read_only(&self) -> bool {
        (self.attributes.0 & 0x01 ) != 0
    }
        
    /// Whether the entry should be "hidden" from directory traversals.
    fn hidden(&self) -> bool {
        (self.attributes.0 & 0x02) != 0
    }

    /// The timestamp when the entry was created.
    fn created(&self) -> Timestamp {
        self.created
    }

    /// The timestamp for the entry's last access.
    fn accessed(&self) -> Timestamp {
        self.accessed
    }

    /// The timestamp for the entry's last modification.
    fn modified(&self) -> Self::Timestamp {
        self.modified
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Attributes: {}\ncreated: {}\nmodified:{}\naccessed: {}", self.attributes.0 , &{self.created.date.0} , &{self.modified.date.0}, &{self.accessed.date.0})
    }
}
