use crate::domain::tools::ListedEntry;
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

pub struct ReadHandle {
    pub file: File,
    _held: Vec<File>,
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
    use super::{ListedEntry, ReadHandle, SafeFileError};
    use crate::domain::tools::ListedKind;
    use std::ffi::{CStr, CString};
    use std::fs::File;
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
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
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
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

    #[cfg(target_os = "macos")]
    fn clear_errno() {
        unsafe { *libc::__error() = 0 };
    }

    #[cfg(not(target_os = "macos"))]
    fn clear_errno() {
        unsafe { *libc::__errno_location() = 0 };
    }

    fn kind_of(stream: *mut libc::DIR, name: &CStr, d_type: u8) -> ListedKind {
        match d_type {
            libc::DT_REG => ListedKind::File,
            libc::DT_DIR => ListedKind::Directory,
            libc::DT_LNK => ListedKind::Symlink,
            libc::DT_UNKNOWN => {
                let mut status: libc::stat = unsafe { std::mem::zeroed() };
                let found = unsafe {
                    libc::fstatat(
                        libc::dirfd(stream),
                        name.as_ptr(),
                        &mut status,
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                };
                match (found, status.st_mode & libc::S_IFMT) {
                    (0, libc::S_IFREG) => ListedKind::File,
                    (0, libc::S_IFDIR) => ListedKind::Directory,
                    (0, libc::S_IFLNK) => ListedKind::Symlink,
                    _ => ListedKind::Other,
                }
            }
            _ => ListedKind::Other,
        }
    }

    fn read_entries(dir: OwnedFd, limit: usize) -> Result<(Vec<ListedEntry>, bool), SafeFileError> {
        let raw = dir.into_raw_fd();
        let stream = unsafe { libc::fdopendir(raw) };
        if stream.is_null() {
            let error = io::Error::last_os_error();
            unsafe { libc::close(raw) };
            return Err(classify(error));
        }
        let mut entries = Vec::new();
        let mut cut = false;
        let mut failure = None;
        loop {
            clear_errno();
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                let error = io::Error::last_os_error();
                if error.raw_os_error().is_some_and(|code| code != 0) {
                    failure = Some(error);
                }
                break;
            }
            let (name, d_type) =
                unsafe { (CStr::from_ptr((*entry).d_name.as_ptr()), (*entry).d_type) };
            if matches!(name.to_bytes(), b"." | b"..") {
                continue;
            }
            if entries.len() == limit {
                cut = true;
                break;
            }
            entries.push(ListedEntry {
                name: String::from_utf8_lossy(name.to_bytes()).into_owned(),
                kind: kind_of(stream, name, d_type),
            });
        }
        unsafe { libc::closedir(stream) };
        match failure {
            Some(error) => Err(classify(error)),
            None => Ok((entries, cut)),
        }
    }

    pub fn list_dir(
        root: &Path,
        relative: &str,
        limit: usize,
    ) -> Result<(Vec<ListedEntry>, bool), SafeFileError> {
        let mut dir = open_root(root)?;
        if !relative.is_empty() {
            for part in relative.split('/') {
                dir = open_at(&dir, part, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
            }
        }
        read_entries(dir, limit)
    }

    pub fn open_for_read(root: &Path, relative: &str) -> Result<ReadHandle, SafeFileError> {
        Ok(ReadHandle {
            file: open_source(root, relative)?,
            _held: Vec::new(),
        })
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
    use super::{ReadHandle, SafeFileError};
    use crate::domain::tools::{ListedEntry, ListedKind};
    use std::fs::{File, OpenOptions};
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
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

    fn refused(path: &Path, error: std::io::Error) -> SafeFileError {
        let folder = error.kind() == std::io::ErrorKind::PermissionDenied
            && std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir());
        if folder {
            SafeFileError::NotAFile
        } else {
            SafeFileError::Io(error)
        }
    }

    pub fn open_source(root: &Path, relative: &str) -> Result<File, SafeFileError> {
        let path = walk(root, relative, false)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&path)
            .map_err(|error| refused(&path, error))?;
        let metadata = file.metadata()?;
        if metadata.file_type().is_symlink() {
            return Err(SafeFileError::SymbolicLink);
        }
        if !metadata.is_file() {
            return Err(SafeFileError::NotAFile);
        }
        Ok(file)
    }

    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    const FILE_SHARE_READ_WRITE: u32 = 0x1 | 0x2;

    pub(super) fn hold(path: &Path) -> Result<File, SafeFileError> {
        let folder = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = folder.metadata()?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(SafeFileError::SymbolicLink);
        }
        if !metadata.is_dir() {
            return Err(SafeFileError::NotAFile);
        }
        Ok(folder)
    }

    pub fn list_dir(
        root: &Path,
        relative: &str,
        limit: usize,
    ) -> Result<(Vec<ListedEntry>, bool), SafeFileError> {
        let mut path = root.to_path_buf();
        let mut held = vec![hold(&path)?];
        if !relative.is_empty() {
            for part in relative.split('/') {
                path.push(part);
                held.push(hold(&path)?);
            }
        }
        let mut entries = Vec::new();
        let mut cut = false;
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            if entries.len() == limit {
                cut = true;
                break;
            }
            let kind = match entry.file_type() {
                Ok(kind) if kind.is_symlink() => ListedKind::Symlink,
                Ok(kind) if kind.is_dir() => ListedKind::Directory,
                Ok(kind) if kind.is_file() => ListedKind::File,
                _ => ListedKind::Other,
            };
            entries.push(ListedEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                kind,
            });
        }
        drop(held);
        Ok((entries, cut))
    }

    pub fn open_for_read(root: &Path, relative: &str) -> Result<ReadHandle, SafeFileError> {
        let mut path = root.to_path_buf();
        let mut held = vec![hold(&path)?];
        let mut parts: Vec<&str> = relative.split('/').collect();
        let last = parts.pop().ok_or(SafeFileError::NotAFile)?;
        for part in parts {
            path.push(part);
            held.push(hold(&path)?);
        }
        path.push(last);
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&path)
            .map_err(|error| refused(&path, error))?;
        let metadata = file.metadata()?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(SafeFileError::SymbolicLink);
        }
        if !metadata.is_file() {
            return Err(SafeFileError::NotAFile);
        }
        Ok(ReadHandle { file, _held: held })
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

pub use platform::{create_destination, list_dir, mode, open_for_read, open_source};

pub fn exists(root: &Path, relative: &str) -> bool {
    std::fs::symlink_metadata(root.join(relative)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tools::ListedKind;

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

    #[test]
    fn lists_a_folder_with_its_kinds_and_stops_at_the_limit() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("src/nested")).unwrap();
        std::fs::write(root.path().join("src/a.txt"), "x").unwrap();
        std::fs::write(root.path().join("src/b.txt"), "x").unwrap();

        let (mut entries, cut) = list_dir(root.path(), "src", 10).unwrap();
        entries.sort_by(|left, right| left.name.cmp(&right.name));

        assert!(!cut);
        assert_eq!(
            entries
                .iter()
                .map(|entry| (entry.name.as_str(), entry.kind))
                .collect::<Vec<_>>(),
            vec![
                ("a.txt", ListedKind::File),
                ("b.txt", ListedKind::File),
                ("nested", ListedKind::Directory)
            ]
        );
        assert_eq!(list_dir(root.path(), "src", 2).unwrap().0.len(), 2);
        assert!(list_dir(root.path(), "src", 2).unwrap().1);
        assert_eq!(list_dir(root.path(), "", 10).unwrap().0.len(), 1);
        assert!(matches!(
            list_dir(root.path(), "src/a.txt", 10),
            Err(SafeFileError::NotAFile)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn listing_never_follows_a_symbolic_link_in_any_folder_or_the_root() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "x").unwrap();
        std::fs::create_dir(root.path().join("real")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("dir")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("real/up")).unwrap();
        let link_root = tempfile::tempdir().unwrap();
        let link = link_root.path().join("root-link");
        std::os::unix::fs::symlink(root.path(), &link).unwrap();

        for relative in ["dir", "dir/", "real/up", "real/up/secret"] {
            assert!(list_dir(root.path(), relative, 10).is_err(), "{relative}");
        }
        assert!(list_dir(&link, "", 10).is_err());
        let (entries, _) = list_dir(root.path(), "", 10).unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| entry.name == "dir" && entry.kind == ListedKind::Symlink)
        );
    }

    #[cfg(windows)]
    fn junction(link: &Path, target: &Path) {
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed");
    }

    #[cfg(windows)]
    #[test]
    fn junctions_are_never_followed_in_the_file_any_folder_or_the_root() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "x").unwrap();
        std::fs::create_dir(root.path().join("real")).unwrap();
        junction(&root.path().join("dir"), outside.path());
        junction(&root.path().join("real").join("up"), outside.path());
        let alias = tempfile::tempdir().unwrap();
        let alias_root = alias.path().join("root-link");
        junction(&alias_root, root.path());

        for relative in ["dir", "real/up", "real/up/secret.txt"] {
            assert!(list_dir(root.path(), relative, 10).is_err(), "{relative}");
        }
        for relative in ["dir/secret.txt", "real/up/secret.txt"] {
            assert!(open_for_read(root.path(), relative).is_err(), "{relative}");
        }
        assert!(list_dir(&alias_root, "", 10).is_err());
        assert!(open_for_read(&alias_root, "real").is_err());
        let (entries, _) = list_dir(root.path(), "", 10).unwrap();
        assert!(entries.iter().any(|entry| entry.name == "dir"
            && matches!(entry.kind, ListedKind::Symlink | ListedKind::Other)));
    }

    #[cfg(windows)]
    #[test]
    fn symbolic_links_are_never_followed_when_the_platform_allows_creating_them() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "x").unwrap();
        let file_link = std::os::windows::fs::symlink_file(
            outside.path().join("secret.txt"),
            root.path().join("link.txt"),
        );
        let folder_link =
            std::os::windows::fs::symlink_dir(outside.path(), root.path().join("linked"));
        if file_link.is_err() || folder_link.is_err() {
            return;
        }

        assert!(open_for_read(root.path(), "link.txt").is_err());
        assert!(open_for_read(root.path(), "linked/secret.txt").is_err());
        assert!(list_dir(root.path(), "linked", 10).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn an_open_folder_cannot_be_renamed_away_while_it_is_held() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("dir")).unwrap();
        let held = platform::hold(&root.path().join("dir")).unwrap();

        let renamed = std::fs::rename(root.path().join("dir"), root.path().join("moved"));

        assert!(renamed.is_err());
        drop(held);
        assert!(std::fs::rename(root.path().join("dir"), root.path().join("moved")).is_ok());
    }
}
