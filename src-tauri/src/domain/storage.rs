use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageIssue {
    Network,
    Removable,
    Synced,
    Unknown,
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
const NETWORK_FILESYSTEMS: &[&str] = &[
    "nfs",
    "nfs4",
    "cifs",
    "smb",
    "smb2",
    "smb3",
    "smbfs",
    "afpfs",
    "webdav",
    "davfs",
    "sshfs",
    "fuse.sshfs",
    "9p",
    "ceph",
    "glusterfs",
    "fuse.glusterfs",
    "afs",
    "lustre",
    "ncpfs",
    "fuse.rclone",
    "fuse.s3fs",
];

#[cfg(any(target_os = "linux", target_os = "macos", test))]
const LOCAL_FILESYSTEMS: &[&str] = &[
    "ext2", "ext3", "ext4", "xfs", "btrfs", "f2fs", "jfs", "reiserfs", "bcachefs", "nilfs2",
    "hfsplus", "apfs", "vfat", "exfat", "ntfs", "ntfs3", "fuseblk",
];

#[cfg(any(target_os = "linux", target_os = "macos", test))]
pub fn filesystem_issue(fstype: &str) -> Option<StorageIssue> {
    if NETWORK_FILESYSTEMS.contains(&fstype) {
        Some(StorageIssue::Network)
    } else if LOCAL_FILESYSTEMS.contains(&fstype) {
        None
    } else {
        Some(StorageIssue::Unknown)
    }
}

#[cfg(any(target_os = "linux", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub mount_point: String,
    pub fstype: String,
    pub source: String,
    pub major: u32,
    pub minor: u32,
}

#[cfg(any(target_os = "linux", test))]
fn unescape(field: &str) -> String {
    let mut text = String::with_capacity(field.len());
    let mut rest = field;
    while let Some(index) = rest.find('\\') {
        text.push_str(&rest[..index]);
        let code = rest.get(index + 1..index + 4);
        match code.and_then(|digits| u8::from_str_radix(digits, 8).ok()) {
            Some(byte) => {
                text.push(byte as char);
                rest = &rest[index + 4..];
            }
            None => {
                text.push('\\');
                rest = &rest[index + 1..];
            }
        }
    }
    text.push_str(rest);
    text
}

#[cfg(any(target_os = "linux", test))]
fn parse_mount(line: &str) -> Option<Mount> {
    let (before, after) = line.split_once(" - ")?;
    let fields: Vec<&str> = before.split(' ').collect();
    let (major, minor) = fields.get(2)?.split_once(':')?;
    let mut after = after.split(' ');
    Some(Mount {
        mount_point: unescape(fields.get(4)?),
        fstype: after.next()?.to_string(),
        source: unescape(after.next()?),
        major: major.parse().ok()?,
        minor: minor.parse().ok()?,
    })
}

#[cfg(any(target_os = "linux", test))]
pub fn mount_for(mountinfo: &str, path: &Path) -> Option<Mount> {
    mountinfo
        .lines()
        .filter_map(parse_mount)
        .filter(|mount| path.starts_with(&mount.mount_point))
        .max_by_key(|mount| Path::new(&mount.mount_point).components().count())
}

#[cfg(any(target_os = "macos", test))]
pub fn plist_bool(xml: &str, key: &str) -> Option<bool> {
    let marker = format!("<key>{key}</key>");
    let rest = xml[xml.find(&marker)? + marker.len()..].trim_start();
    if rest.starts_with("<true/>") {
        Some(true)
    } else if rest.starts_with("<false/>") {
        Some(false)
    } else {
        None
    }
}

pub fn dropbox_paths(info_json: &str) -> Vec<PathBuf> {
    let Ok(serde_json::Value::Object(accounts)) = serde_json::from_str(info_json) else {
        return Vec::new();
    };
    accounts
        .values()
        .filter_map(|account| account.get("path")?.as_str())
        .map(PathBuf::from)
        .collect()
}

pub fn is_inside(path: &Path, roots: &[PathBuf], case_sensitive: bool) -> bool {
    let fold = |path: &Path| {
        let text = path.to_string_lossy().replace('\\', "/");
        if case_sensitive {
            text
        } else {
            text.to_lowercase()
        }
    };
    let path = PathBuf::from(fold(path));
    roots
        .iter()
        .any(|root| path.starts_with(PathBuf::from(fold(root))))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTINFO: &str = "\
22 1 8:1 / / rw,relatime shared:1 - ext4 /dev/sda1 rw
30 22 0:26 / /home rw,relatime shared:2 - btrfs /dev/nvme0n1p2 rw
31 22 0:40 / /mnt/share rw shared:3 - nfs4 server:/export rw
32 22 8:17 / /media/ana/USB\\040Stick rw shared:4 - vfat /dev/sdb1 rw
33 22 0:30 / /tmp rw shared:5 - tmpfs tmpfs rw
";

    #[test]
    fn classifies_filesystems() {
        assert_eq!(filesystem_issue("ext4"), None);
        assert_eq!(filesystem_issue("apfs"), None);
        assert_eq!(filesystem_issue("nfs4"), Some(StorageIssue::Network));
        assert_eq!(filesystem_issue("smbfs"), Some(StorageIssue::Network));
        assert_eq!(filesystem_issue("tmpfs"), Some(StorageIssue::Unknown));
        assert_eq!(
            filesystem_issue("fuse.unknown"),
            Some(StorageIssue::Unknown)
        );
    }

    #[test]
    fn finds_the_deepest_mount_of_a_path() {
        let home = mount_for(MOUNTINFO, Path::new("/home/ana/code")).unwrap();
        let root = mount_for(MOUNTINFO, Path::new("/homework")).unwrap();

        assert_eq!(home.fstype, "btrfs");
        assert_eq!(home.source, "/dev/nvme0n1p2");
        assert_eq!((home.major, home.minor), (0, 26));
        assert_eq!(root.mount_point, "/");
        assert_eq!(root.fstype, "ext4");
        assert_eq!(
            mount_for(MOUNTINFO, Path::new("/mnt/share/repo"))
                .unwrap()
                .fstype,
            "nfs4"
        );
    }

    #[test]
    fn unescapes_spaces_in_mount_points() {
        let mount = mount_for(MOUNTINFO, Path::new("/media/ana/USB Stick/repo")).unwrap();

        assert_eq!(mount.mount_point, "/media/ana/USB Stick");
        assert_eq!((mount.major, mount.minor), (8, 17));
    }

    #[test]
    fn reads_booleans_from_a_plist() {
        let xml =
            "<dict>\n\t<key>Ejectable</key>\n\t<false/>\n\t<key>Internal</key>\n\t<true/>\n</dict>";

        assert_eq!(plist_bool(xml, "Internal"), Some(true));
        assert_eq!(plist_bool(xml, "Ejectable"), Some(false));
        assert_eq!(plist_bool(xml, "RemovableMedia"), None);
    }

    #[test]
    fn reads_the_dropbox_folders() {
        let json = r#"{"personal": {"path": "/Users/ana/Dropbox", "host": 1}, "business": {"path": "/Users/ana/Dropbox (Acme)"}}"#;

        let mut paths = dropbox_paths(json);
        paths.sort();

        assert_eq!(
            paths,
            [
                PathBuf::from("/Users/ana/Dropbox"),
                PathBuf::from("/Users/ana/Dropbox (Acme)")
            ]
        );
        assert!(dropbox_paths("not json").is_empty());
    }

    #[test]
    fn checks_containment_by_whole_components() {
        let roots = [PathBuf::from("/Users/ana/Dropbox")];

        assert!(is_inside(
            Path::new("/Users/ana/Dropbox/repo"),
            &roots,
            true
        ));
        assert!(is_inside(Path::new("/Users/ana/Dropbox"), &roots, true));
        assert!(!is_inside(
            Path::new("/Users/ana/Dropbox2/repo"),
            &roots,
            true
        ));
        assert!(!is_inside(
            Path::new("/users/ana/dropbox/repo"),
            &roots,
            true
        ));
        assert!(is_inside(
            Path::new("/users/ana/dropbox/repo"),
            &roots,
            false
        ));
        assert!(is_inside(
            Path::new("C:\\Users\\Ana\\OneDrive\\repo"),
            &[PathBuf::from("C:\\Users\\ana\\OneDrive")],
            false
        ));
    }
}
