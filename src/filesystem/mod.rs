use alloc::boxed::Box;

#[derive(Debug)]
pub struct FileDescriptor<FileSystemData> {
    pub data: Box<FileSystemData>
}

pub trait FileSystem {
    type FileSystemData;
    /// NOTES:
    ///
    /// * Relative paths start at the filesystem's root; they are identical to absolute paths
    fn open_file(&mut self, path: &str) -> Result<FileDescriptor<Self::FileSystemData>, OpenFileError>;

    fn close_file(&mut self, descriptor: FileDescriptor<Self::FileSystemData>);

    fn read_file(&mut self, descriptor: &mut FileDescriptor<Self::FileSystemData>, buf: &mut [u8], count: usize);

    fn seek_file(&mut self, descriptor: &mut FileDescriptor<Self::FileSystemData>, location: u64);

    fn get_size(&mut self, descriptor: &mut FileDescriptor<Self::FileSystemData>) -> u64;
}

/// An error returned from opening a file.
#[derive(Debug)]
pub enum OpenFileError {
    /// The opened path is too long to be valid for this filesystem.
    PathTooLong,
    /// One of the path's components is too long to be valid for this filesystem.
    ComponentTooLong,
    /// The opened path cannot be converted into the proper charset.
    InvalidCharset,
    /// The opened file was not found on the filesystem.
    FileNotFound,
    /// An error occurred while reading from this filesystem's device.
    DeviceError,
    /// Access was denied to this file
    AccessDenied,
    /// The filesystem state has been corrupted.
    FileSystemCorrupted,
    /// A directory on the path to the opened file was not found on the filesystem.
    DirectoryNotFound,
    /// Tried to open a file as a directory.
    IsFile,
    /// Tried to open a directory as a normal file.
    IsDirectory,
}
