use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};
use core::fmt;

#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    FILE(File<HANDLE>),
    DIR(Dir<HANDLE>),
}


impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    type File= File<HANDLE>;
    type Dir= Dir<HANDLE>;
    type Metadata= Metadata;

    /// The name of the file or directory corresponding to this entry.
    fn name(&self) -> &str {
        match self {
            Entry::FILE(f) => &f.file_name,
            Entry::DIR(d) => &d.file_name,
        }
    }

    /// The metadata associated with the entry.
    fn metadata(&self) -> &Self::Metadata{
        match self {
            Entry::FILE(f) => &f.metadata,
            Entry::DIR(d) => &d.metadata,
        }
    }

    /// If `self` is a file, returns `Some` of a reference to the file.
    /// Otherwise returns `None`.
    fn as_file(&self) -> Option<&self::File<HANDLE>>{
        match self {
            Entry::FILE(f) => Some(f),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of a reference to the
    /// directory. Otherwise returns `None`.
    fn as_dir(&self) -> Option<&self::Dir<HANDLE>>{
        match self {
            Entry::DIR(d)=> Some(d),
            _ => None,
        }
    }

    /// If `self` is a file, returns `Some` of the file. Otherwise returns
    /// `None`.
    fn into_file(self) -> Option<self::File<HANDLE>>{
        match self {
            Entry::FILE(f) => Some(f),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of the directory. Otherwise
    /// returns `None`.
    fn into_dir(self) -> Option<self::Dir<HANDLE>>{
        match self {
            Entry::DIR(d) => Some(d),
            _ => None,
        }
    }

    /// Returns `true` if this entry is a file or `false` otherwise.
    fn is_file(&self) -> bool {
        self.as_file().is_some()
    }

    /// Returns `true` if this entry is a directory or `false` otherwise.
    fn is_dir(&self) -> bool {
        self.as_dir().is_some()
    }
}
