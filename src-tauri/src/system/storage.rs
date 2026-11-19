use crate::domain::storage::{StorageIssue, dropbox_paths, is_inside};
use std::path::{Path, PathBuf};

pub async fn check(path: &Path) -> Result<(), StorageIssue> {
    let existing = path
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .ok_or(StorageIssue::Unknown)?;
    let canonical = std::fs::canonicalize(existing).map_err(|_| StorageIssue::Unknown)?;
    let home = std::env::home_dir().ok_or(StorageIssue::Unknown)?;
    let (synced, uncertain) = synced_roots(&home);
    let case_sensitive = cfg!(target_os = "linux");
    if is_inside(&canonical, &synced, case_sensitive) {
        return Err(StorageIssue::Synced);
    }
    if is_inside(&canonical, &uncertain, case_sensitive) {
        return Err(StorageIssue::Unknown);
    }
    volume(&canonical).await
}

fn synced_roots(home: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut uncertain = Vec::new();
    let mut roots = vec![
        home.join("Dropbox"),
        home.join("Google Drive"),
        home.join("OneDrive"),
        home.join("iCloudDrive"),
    ];
    let mut dropbox_info = vec![home.join(".dropbox").join("info.json")];
    for variable in ["OneDrive", "OneDriveCommercial", "OneDriveConsumer"] {
        if let Some(value) = std::env::var_os(variable) {
            roots.push(PathBuf::from(value));
        }
    }
    for variable in ["LOCALAPPDATA", "APPDATA"] {
        if let Some(value) = std::env::var_os(variable) {
            dropbox_info.push(PathBuf::from(value).join("Dropbox").join("info.json"));
        }
    }
    for info in dropbox_info {
        if let Ok(text) = std::fs::read_to_string(info) {
            roots.extend(dropbox_paths(&text));
        }
    }
    if cfg!(target_os = "macos") {
        let library = home.join("Library");
        roots.push(library.join("Mobile Documents"));
        roots.push(library.join("CloudStorage"));
        let cloud_docs = library.join("Mobile Documents").join("com~apple~CloudDocs");
        for folder in ["Desktop", "Documents"] {
            match std::fs::symlink_metadata(cloud_docs.join(folder)) {
                Ok(_) => roots.push(home.join(folder)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => uncertain.push(home.join(folder)),
            }
        }
    }
    (roots, uncertain)
}

#[cfg(target_os = "macos")]
async fn volume(path: &Path) -> Result<(), StorageIssue> {
    use crate::domain::storage::{filesystem_issue, plist_bool};
    use std::ffi::{CStr, CString};
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| StorageIssue::Unknown)?;
    let mut stats: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c_path.as_ptr(), &mut stats) } != 0 {
        return Err(StorageIssue::Unknown);
    }
    if stats.f_flags & libc::MNT_LOCAL as u32 == 0 {
        return Err(StorageIssue::Network);
    }
    let fstype = unsafe { CStr::from_ptr(stats.f_fstypename.as_ptr()) }.to_string_lossy();
    if let Some(issue) = filesystem_issue(&fstype) {
        return Err(issue);
    }
    let mount = unsafe { CStr::from_ptr(stats.f_mntonname.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    let output = tokio::process::Command::new("diskutil")
        .args(["info", "-plist"])
        .arg(&mount)
        .output()
        .await
        .map_err(|_| StorageIssue::Unknown)?;
    if !output.status.success() {
        return Err(StorageIssue::Unknown);
    }
    let xml = String::from_utf8_lossy(&output.stdout);
    match (
        plist_bool(&xml, "Internal"),
        plist_bool(&xml, "RemovableMedia"),
        plist_bool(&xml, "Ejectable"),
    ) {
        (Some(true), Some(false), Some(false)) => Ok(()),
        (Some(_), Some(_), Some(_)) => Err(StorageIssue::Removable),
        _ => Err(StorageIssue::Unknown),
    }
}

#[cfg(target_os = "linux")]
async fn volume(path: &Path) -> Result<(), StorageIssue> {
    use crate::domain::storage::{filesystem_issue, mount_for};
    use std::os::unix::fs::MetadataExt;

    let mountinfo =
        std::fs::read_to_string("/proc/self/mountinfo").map_err(|_| StorageIssue::Unknown)?;
    let mount = mount_for(&mountinfo, path).ok_or(StorageIssue::Unknown)?;
    if let Some(issue) = filesystem_issue(&mount.fstype) {
        return Err(issue);
    }
    let (major, minor) = if mount.major != 0 {
        (mount.major, mount.minor)
    } else if mount.source.starts_with("/dev/") {
        let device = std::fs::metadata(&mount.source)
            .map_err(|_| StorageIssue::Unknown)?
            .rdev();
        (libc::major(device), libc::minor(device))
    } else {
        return Err(StorageIssue::Unknown);
    };
    block_device(major, minor, 0)
}

#[cfg(target_os = "linux")]
fn block_device(major: u32, minor: u32, depth: usize) -> Result<(), StorageIssue> {
    if depth > 8 {
        return Err(StorageIssue::Unknown);
    }
    let device = std::fs::canonicalize(format!("/sys/dev/block/{major}:{minor}"))
        .map_err(|_| StorageIssue::Unknown)?;
    let text = device.to_string_lossy();
    if text.contains("/usb") || text.contains("/mmc") || text.contains("/firewire") {
        return Err(StorageIssue::Removable);
    }
    if text.contains("/virtual/") {
        let slaves: Vec<_> = std::fs::read_dir(device.join("slaves"))
            .map_err(|_| StorageIssue::Unknown)?
            .flatten()
            .collect();
        if slaves.is_empty() {
            return Err(StorageIssue::Unknown);
        }
        for slave in slaves {
            let numbers = std::fs::read_to_string(slave.path().join("dev"))
                .map_err(|_| StorageIssue::Unknown)?;
            let (major, minor) = numbers
                .trim()
                .split_once(':')
                .ok_or(StorageIssue::Unknown)?;
            block_device(
                major.parse().map_err(|_| StorageIssue::Unknown)?,
                minor.parse().map_err(|_| StorageIssue::Unknown)?,
                depth + 1,
            )?;
        }
        return Ok(());
    }
    let disk = if device.join("partition").exists() {
        device.parent().ok_or(StorageIssue::Unknown)?.to_path_buf()
    } else {
        device
    };
    match std::fs::read_to_string(disk.join("removable")) {
        Ok(flag) if flag.trim() == "0" => Ok(()),
        Ok(_) => Err(StorageIssue::Removable),
        Err(_) => Err(StorageIssue::Unknown),
    }
}

#[cfg(windows)]
async fn volume(path: &Path) -> Result<(), StorageIssue> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        BusType1394, BusTypeMmc, BusTypeSd, BusTypeUsb, CreateFileW, FILE_SHARE_READ,
        FILE_SHARE_WRITE, GetDriveTypeW, GetVolumePathNameW, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery, STORAGE_DEVICE_DESCRIPTOR,
        STORAGE_PROPERTY_QUERY, StorageDeviceProperty,
    };

    const DRIVE_FIXED: u32 = 3;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_CDROM: u32 = 5;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut buffer = [0u16; 512];
    if unsafe { GetVolumePathNameW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) } == 0 {
        return Err(StorageIssue::Unknown);
    }
    let length = buffer.iter().position(|unit| *unit == 0).unwrap_or(0);
    let root = std::ffi::OsString::from_wide(&buffer[..length])
        .to_string_lossy()
        .into_owned();
    let root = root.strip_prefix(r"\\?\").unwrap_or(&root).to_string();
    if root.starts_with(r"\\") {
        return Err(StorageIssue::Network);
    }
    let root_wide: Vec<u16> = std::ffi::OsStr::new(&root)
        .encode_wide()
        .chain([0])
        .collect();
    match unsafe { GetDriveTypeW(root_wide.as_ptr()) } {
        DRIVE_FIXED => {}
        DRIVE_REMOTE => return Err(StorageIssue::Network),
        DRIVE_REMOVABLE | DRIVE_CDROM => return Err(StorageIssue::Removable),
        _ => return Err(StorageIssue::Unknown),
    }
    let letter = root.trim_end_matches('\\');
    if letter.len() != 2 || !letter.ends_with(':') {
        return Err(StorageIssue::Unknown);
    }
    let device: Vec<u16> = std::ffi::OsStr::new(&format!(r"\\.\{letter}"))
        .encode_wide()
        .chain([0])
        .collect();
    let handle = unsafe {
        CreateFileW(
            device.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(StorageIssue::Unknown);
    }
    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0],
    };
    let mut descriptor = [0u8; 1024];
    let mut returned = 0u32;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            (&query as *const STORAGE_PROPERTY_QUERY).cast(),
            std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            descriptor.as_mut_ptr().cast(),
            descriptor.len() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    unsafe { CloseHandle(handle) };
    if ok == 0 || (returned as usize) < std::mem::size_of::<STORAGE_DEVICE_DESCRIPTOR>() {
        return Err(StorageIssue::Unknown);
    }
    let descriptor: STORAGE_DEVICE_DESCRIPTOR =
        unsafe { std::ptr::read_unaligned(descriptor.as_ptr().cast()) };
    if descriptor.RemovableMedia
        || [BusTypeUsb, BusType1394, BusTypeSd, BusTypeMmc].contains(&descriptor.BusType)
    {
        return Err(StorageIssue::Removable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn accepts_the_local_disk_of_the_test_machine() {
        let dir = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();

        assert_eq!(check(dir.path()).await, Ok(()));
        assert_eq!(check(&dir.path().join("not/created/yet")).await, Ok(()));
    }

    #[tokio::test]
    async fn rejects_known_synced_folders() {
        let home = std::env::home_dir().unwrap();
        let (synced, _) = synced_roots(&home);

        assert!(is_inside(&home.join("Dropbox").join("repo"), &synced, true));
    }
}
