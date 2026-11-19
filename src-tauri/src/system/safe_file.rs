use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum SafeFileError {
    #[error("symbolic link")]
    SymbolicLink,
    #[error("not a regular file")]
    NotAFile,
    #[error("already exists")]
    Exists,
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub fn copy_limited(source: &mut File, destination: &mut File, limit: u64) -> io::Result<u64> {
    use io::Read;
    let copied = io::copy(&mut source.take(limit + 1), destination)?;
    if copied > limit {
        return Err(io::Error::new(io::ErrorKind::FileTooLarge, "too large"));
    }
    destination.sync_all()?;
    Ok(copied)
}

#[cfg(unix)]
mod platform {
    use super::SafeFileError;
    use std::ffi::CString;
    use std::fs::File;
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    fn name(part: &str) -> Result<CString, SafeFileError> {
        CString::new(part).map_err(|_| SafeFileError::NotAFile)
    }

    fn classify(error: io::Error) -> SafeFileError {
        match error.raw_os_error() {
            Some(libc::ELOOP) => SafeFileError::SymbolicLink,
            Some(libc::ENOTDIR) => SafeFileError::NotAFile,
            Some(libc::EEXIST) => SafeFileError::Exists,
            _ => SafeFileError::Io(error),
        }
    }

    fn open_root(path: &Path) -> Result<OwnedFd, SafeFileError> {
        let path =
            CString::new(path.as_os_str().as_bytes()).map_err(|_| SafeFileError::NotAFile)?;
        let fd = unsafe {
            libc::open(
                path.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(classify(io::Error::last_os_error()));
        }
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    fn open_at(dir: &OwnedFd, part: &str, flags: i32, mode: u32) -> Result<OwnedFd, SafeFileError> {
        let part = name(part)?;
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                part.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                mode as libc::c_uint,
            )
        };
        if fd < 0 {
            return Err(classify(io::Error::last_os_error()));
        }
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    fn walk<'a>(
        root: &Path,
        relative: &'a str,
        create: bool,
    ) -> Result<(OwnedFd, &'a str), SafeFileError> {
        let mut parts: Vec<&str> = relative.split('/').collect();
        let last = parts.pop().ok_or(SafeFileError::NotAFile)?;
        let mut dir = open_root(root)?;
        for part in parts {
            let flags = libc::O_RDONLY | libc::O_DIRECTORY;
            dir = match open_at(&dir, part, flags, 0) {
                Err(SafeFileError::Io(error))
                    if create && error.kind() == io::ErrorKind::NotFound =>
                {
                    let c_part = name(part)?;
                    if unsafe { libc::mkdirat(dir.as_raw_fd(), c_part.as_ptr(), 0o755) } != 0 {
                        return Err(classify(io::Error::last_os_error()));
                    }
                    open_at(&dir, part, flags, 0)?
                }
                other => other?,
            };
        }
        Ok((dir, last))
    }

    pub fn open_source(root: &Path, relative: &str) -> Result<File, SafeFileError> {
        let (dir, last) = walk(root, relative, false)?;
        let file = File::from(open_at(&dir, last, libc::O_RDONLY | libc::O_NONBLOCK, 0)?);
        if !file.metadata()?.is_file() {
            return Err(SafeFileError::NotAFile);
        }
        Ok(file)
    }

    pub fn create_destination(
        root: &Path,
        relative: &str,
        mode: u32,
    ) -> Result<File, SafeFileError> {
        let (dir, last) = walk(root, relative, true)?;
        let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL;
        Ok(File::from(open_at(&dir, last, flags, mode & 0o777)?))
    }

    pub fn mode(file: &File) -> std::io::Result<u32> {
        use std::os::unix::fs::PermissionsExt;
        Ok(file.metadata()?.permissions().mode())
    }
}

#[cfg(windows)]
mod platform {
    use super::SafeFileError;
    use std::fs::{File, OpenOptions};
    use std::os::windows::fs::OpenOptionsExt;
    use std::path::{Path, PathBuf};

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

    fn walk(root: &Path, relative: &str, create: bool) -> Result<PathBuf, SafeFileError> {
        let mut path = root.to_path_buf();
        let mut parts: Vec<&str> = relative.split('/').collect();
        let last = parts.pop().ok_or(SafeFileError::NotAFile)?;
        for part in parts {
            path.push(part);
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(SafeFileError::SymbolicLink);
                }
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => return Err(SafeFileError::NotAFile),
                Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                    std::fs::create_dir(&path)?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        path.push(last);
        Ok(path)
    }

    pub fn open_source(root: &Path, relative: &str) -> Result<File, SafeFileError> {
        let path = walk(root, relative, false)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = file.metadata()?;
        if metadata.file_type().is_symlink() {
            return Err(SafeFileError::SymbolicLink);
        }
        if !metadata.is_file() {
            return Err(SafeFileError::NotAFile);
        }
        Ok(file)
    }

    pub fn create_destination(
        root: &Path,
        relative: &str,
        _mode: u32,
    ) -> Result<File, SafeFileError> {
        let path = walk(root, relative, true)?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::AlreadyExists => SafeFileError::Exists,
                _ => SafeFileError::Io(error),
            })
    }

    pub fn mode(_file: &File) -> std::io::Result<u32> {
        Ok(0o644)
    }
}

pub use platform::{create_destination, mode, open_source};

pub fn exists(root: &Path, relative: &str) -> bool {
    std::fs::symlink_metadata(root.join(relative)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_regular_files_below_the_root() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("config")).unwrap();
        std::fs::write(root.path().join("config/.env"), "A=1").unwrap();

        let mut file = open_source(root.path(), "config/.env").unwrap();
        let mut text = String::new();
        io::Read::read_to_string(&mut file, &mut text).unwrap();

        assert_eq!(text, "A=1");
        assert!(matches!(
            open_source(root.path(), "config"),
            Err(SafeFileError::NotAFile)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symbolic_links_in_the_file_or_any_folder() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "x").unwrap();
        std::os::unix::fs::symlink(outside.path().join("secret"), root.path().join("link"))
            .unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("dir")).unwrap();

        assert!(matches!(
            open_source(root.path(), "link"),
            Err(SafeFileError::SymbolicLink)
        ));
        assert!(matches!(
            open_source(root.path(), "dir/secret"),
            Err(SafeFileError::SymbolicLink | SafeFileError::NotAFile)
        ));
        assert!(matches!(
            create_destination(root.path(), "dir/new", 0o600),
            Err(SafeFileError::SymbolicLink | SafeFileError::NotAFile)
        ));
        assert!(!outside.path().join("new").exists());
    }

    #[test]
    fn creates_missing_folders_and_never_overwrites() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("taken"), "keep").unwrap();

        let mut created = create_destination(root.path(), "a/b/.env", 0o600).unwrap();
        io::Write::write_all(&mut created, b"x").unwrap();

        assert!(root.path().join("a/b/.env").is_file());
        assert!(matches!(
            create_destination(root.path(), "taken", 0o600),
            Err(SafeFileError::Exists)
        ));
        assert_eq!(
            std::fs::read_to_string(root.path().join("taken")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn stops_copies_over_the_limit() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("big"), vec![0u8; 11]).unwrap();
        let mut source = open_source(root.path(), "big").unwrap();
        let mut destination = create_destination(root.path(), "copy", 0o600).unwrap();

        let error = copy_limited(&mut source, &mut destination, 10).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
    }
}
