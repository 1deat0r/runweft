use std::fs::File;
use std::io;
use std::path::{Component, Path, PathBuf};

/// A held operating-system lock for one profile directory.
///
/// Every Runweft coordinator for the profile must acquire this lock before opening
/// the ledger or admitting dispatch. This coordinates cooperating processes; it is
/// not a sandbox against a process that ignores the lock or runs as the same user.
pub struct ProfileOwnership {
    _lock_file: File,
    // Retain the opened directory so later profile operations can be anchored to
    // the same inode instead of resolving the user-supplied path again.
    _profile_dir: File,
}

#[derive(Debug)]
pub enum OwnershipError {
    NotDirectory,
    AlreadyOwned,
    UnsupportedPlatform,
    UnsupportedPath,
    InsecureProfileDirectory,
    InvalidLockFile,
    InvalidDatabaseFile,
    UnsupportedFilesystem(u64),
    DatabaseFilesystemMismatch {
        database_magic: u64,
        profile_magic: u64,
    },
    Io(io::Error),
}

impl std::fmt::Display for OwnershipError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotDirectory => formatter.write_str("profile path is not a directory"),
            Self::AlreadyOwned => formatter.write_str("profile already has a coordinator owner"),
            Self::UnsupportedPlatform => {
                formatter.write_str("profile ownership is not implemented on this platform")
            }
            Self::UnsupportedPath => {
                formatter.write_str("profile path must not contain parent-directory components")
            }
            Self::InsecureProfileDirectory => formatter.write_str(
                "profile path must resolve through trusted directories to a private directory owned by the current user",
            ),
            Self::InvalidLockFile => formatter.write_str(
                "coordinator lock must be a private, single-link regular file owned by the current user",
            ),
            Self::InvalidDatabaseFile => formatter.write_str(
                "ledger database must be a private, single-link regular file owned by the current user",
            ),
            Self::UnsupportedFilesystem(magic) => write!(
                formatter,
                "profile filesystem {magic:#x} is not in the supported local-filesystem set",
            ),
            Self::DatabaseFilesystemMismatch {
                database_magic,
                profile_magic,
            } => write!(
                formatter,
                "database filesystem {database_magic:#x} does not match profile filesystem {profile_magic:#x} on the same device",
            ),
            Self::Io(error) => write!(formatter, "profile ownership I/O failed: {error}"),
        }
    }
}

impl std::error::Error for OwnershipError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::NotDirectory
            | Self::AlreadyOwned
            | Self::UnsupportedPlatform
            | Self::UnsupportedPath
            | Self::InsecureProfileDirectory
            | Self::InvalidLockFile
            | Self::InvalidDatabaseFile
            | Self::UnsupportedFilesystem(_)
            | Self::DatabaseFilesystemMismatch { .. } => None,
        }
    }
}

impl From<io::Error> for OwnershipError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl ProfileOwnership {
    /// Acquires the profile's OS lock and holds it until this value is dropped.
    pub fn acquire(profile_dir: &Path) -> Result<Self, OwnershipError> {
        #[cfg(target_os = "linux")]
        {
            Self::acquire_linux(profile_dir)
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = profile_dir;
            Err(OwnershipError::UnsupportedPlatform)
        }
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn ledger_path(&self) -> PathBuf {
        use std::os::fd::AsRawFd;

        PathBuf::from(format!(
            "/proc/self/fd/{}/ledger.sqlite",
            self._profile_dir.as_raw_fd()
        ))
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn marker_file(&self) -> &File {
        &self._lock_file
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn sync_profile_directory(&self) -> Result<(), OwnershipError> {
        self._profile_dir.sync_all().map_err(OwnershipError::Io)
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn prepare_ledger_file(
        &self,
        create_if_missing: bool,
    ) -> Result<bool, OwnershipError> {
        use rustix::fs::{Mode, OFlags, openat};
        use rustix::process::geteuid;
        use std::os::unix::fs::MetadataExt;

        let flags = OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
        let (file, existed) =
            match openat(&self._profile_dir, "ledger.sqlite", flags, Mode::empty()) {
                Ok(file) => (Some(File::from(file)), true),
                Err(rustix::io::Errno::NOENT) if !create_if_missing => (None, false),
                Err(rustix::io::Errno::NOENT) => match openat(
                    &self._profile_dir,
                    "ledger.sqlite",
                    flags | OFlags::CREATE | OFlags::EXCL,
                    Mode::RUSR | Mode::WUSR,
                ) {
                    Ok(file) => (Some(File::from(file)), false),
                    Err(rustix::io::Errno::EXIST) => (
                        Some(File::from(
                            openat(&self._profile_dir, "ledger.sqlite", flags, Mode::empty())
                                .map_err(as_io)?,
                        )),
                        true,
                    ),
                    Err(error) => return Err(OwnershipError::Io(as_io(error))),
                },
                Err(error) => return Err(OwnershipError::Io(as_io(error))),
            };
        if let Some(file) = file.as_ref() {
            let metadata = file.metadata()?;
            let mode = metadata.mode();
            if !metadata.is_file()
                || metadata.uid() != geteuid().as_raw()
                || metadata.nlink() != 1
                || mode & 0o077 != 0
                || mode & 0o600 != 0o600
            {
                return Err(OwnershipError::InvalidDatabaseFile);
            }

            ensure_same_supported_filesystem(&self._profile_dir, file, metadata.dev())?;
        }
        let mut found_sidecar = false;
        for sidecar in [
            "ledger.sqlite-wal",
            "ledger.sqlite-shm",
            "ledger.sqlite-journal",
        ] {
            let sidecar = match openat(&self._profile_dir, sidecar, flags, Mode::empty()) {
                Ok(file) => File::from(file),
                Err(rustix::io::Errno::NOENT) => continue,
                Err(error) => return Err(OwnershipError::Io(as_io(error))),
            };
            found_sidecar = true;
            let metadata = sidecar.metadata()?;
            if !metadata.is_file()
                || metadata.uid() != geteuid().as_raw()
                || metadata.nlink() != 1
                || metadata.mode() & 0o022 != 0
            {
                return Err(OwnershipError::InvalidDatabaseFile);
            }
            ensure_same_supported_filesystem(&self._profile_dir, &sidecar, metadata.dev())?;
        }

        if !existed && found_sidecar {
            return Err(OwnershipError::InvalidDatabaseFile);
        }

        Ok(existed)
    }

    #[cfg(target_os = "linux")]
    fn acquire_linux(profile_dir: &Path) -> Result<Self, OwnershipError> {
        use rustix::fs::{Mode, OFlags, open, openat};
        use rustix::process::geteuid;
        use std::os::unix::fs::MetadataExt;

        let absolute_path = if profile_dir.is_absolute() {
            profile_dir.to_owned()
        } else {
            std::env::current_dir()?.join(profile_dir)
        };
        let components = absolute_path
            .components()
            .filter_map(|component| match component {
                Component::RootDir | Component::CurDir => None,
                Component::Normal(name) => Some(Ok(name.to_owned())),
                Component::ParentDir | Component::Prefix(_) => {
                    Some(Err(OwnershipError::UnsupportedPath))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        if components.is_empty() {
            return Err(OwnershipError::NotDirectory);
        }

        let directory_flags =
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mut directory = File::from(open("/", directory_flags, Mode::empty()).map_err(as_io)?);
        let effective_uid = geteuid().as_raw();

        for (index, component) in components.iter().enumerate() {
            let next = openat(
                &directory,
                component.as_os_str(),
                directory_flags,
                Mode::empty(),
            )
            .map(File::from)
            .map_err(as_io)?;
            let metadata = next.metadata()?;
            if !metadata.is_dir() {
                return Err(OwnershipError::NotDirectory);
            }
            if index + 1 == components.len() {
                let mode = metadata.mode();
                if metadata.uid() != effective_uid || mode & 0o077 != 0 || mode & 0o700 != 0o700 {
                    return Err(OwnershipError::InsecureProfileDirectory);
                }
            } else {
                let mode = metadata.mode();
                let uid = metadata.uid();
                let trusted_owner = uid == effective_uid || uid == 0;
                let writable_by_others = mode & 0o022 != 0;
                let root_sticky_directory = uid == 0 && mode & 0o1000 != 0;
                if !trusted_owner || (writable_by_others && !root_sticky_directory) {
                    return Err(OwnershipError::InsecureProfileDirectory);
                }
            }
            directory = next;
        }

        let filesystem_type = rustix::fs::fstatfs(&directory).map_err(as_io)?.f_type as u64;
        if !is_supported_local_filesystem(filesystem_type) {
            return Err(OwnershipError::UnsupportedFilesystem(filesystem_type));
        }

        let lock_file = File::from(
            openat(
                &directory,
                "coordinator.lock",
                OFlags::RDWR
                    | OFlags::CREATE
                    | OFlags::NOFOLLOW
                    | OFlags::NONBLOCK
                    | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(as_io)?,
        );
        let lock_metadata = lock_file.metadata()?;
        let lock_mode = lock_metadata.mode();
        if !lock_metadata.is_file()
            || lock_metadata.uid() != effective_uid
            || lock_metadata.nlink() != 1
            || lock_mode & 0o077 != 0
            || lock_mode & 0o600 != 0o600
        {
            return Err(OwnershipError::InvalidLockFile);
        }

        match lock_file.try_lock() {
            Ok(()) => Ok(Self {
                _lock_file: lock_file,
                _profile_dir: directory,
            }),
            Err(std::fs::TryLockError::WouldBlock) => Err(OwnershipError::AlreadyOwned),
            Err(std::fs::TryLockError::Error(error)) => Err(OwnershipError::Io(error)),
        }
    }
}

#[cfg(target_os = "linux")]
fn as_io(error: rustix::io::Errno) -> io::Error {
    error.into()
}

#[cfg(target_os = "linux")]
fn is_supported_local_filesystem(magic: u64) -> bool {
    // Linux magic values from include/uapi/linux/magic.h. Unknown filesystems fail
    // closed so remote or unreviewed locking/WAL semantics are not assumed safe.
    matches!(
        magic,
        0x0000_ef53 // ext2/3/4
            | 0x9123_683e // btrfs
            | 0x5846_5342 // xfs
            | 0xf2f5_2010 // f2fs
            | 0xca45_1a4e // bcachefs
    )
}

#[cfg(target_os = "linux")]
fn ensure_same_supported_filesystem(
    profile_dir: &File,
    file: &File,
    file_device: u64,
) -> Result<(), OwnershipError> {
    use std::os::unix::fs::MetadataExt;

    let profile_magic = rustix::fs::fstatfs(profile_dir).map_err(as_io)?.f_type as u64;
    let file_magic = rustix::fs::fstatfs(file).map_err(as_io)?.f_type as u64;
    let profile_device = profile_dir.metadata()?.dev();
    if !is_supported_database_filesystem(profile_magic, file_magic, file_device == profile_device) {
        return Err(OwnershipError::DatabaseFilesystemMismatch {
            database_magic: file_magic,
            profile_magic,
        });
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn is_supported_database_filesystem(
    profile_magic: u64,
    database_magic: u64,
    same_device: bool,
) -> bool {
    same_device && database_magic == profile_magic && is_supported_local_filesystem(database_magic)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{OwnershipError, ProfileOwnership};
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_PROFILE: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn only_allows_known_local_filesystems() {
        assert!(super::is_supported_local_filesystem(0x0000_ef53));
        assert!(!super::is_supported_local_filesystem(0x0102_1994)); // tmpfs is volatile
        assert!(!super::is_supported_local_filesystem(0x0000_6969)); // NFS
        assert!(!super::is_supported_local_filesystem(0xff53_4d42)); // CIFS
        assert!(!super::is_supported_local_filesystem(0x0102_1997)); // 9P
        assert!(super::is_supported_database_filesystem(
            0x0000_ef53,
            0x0000_ef53,
            true
        ));
        assert!(!super::is_supported_database_filesystem(
            0x0000_ef53,
            0x0102_1994,
            true
        ));
        assert!(!super::is_supported_database_filesystem(
            0x0000_ef53,
            0x0000_ef53,
            false
        ));
    }

    struct TempProfile(PathBuf);

    impl TempProfile {
        fn new() -> Self {
            let sequence = NEXT_PROFILE.fetch_add(1, Ordering::Relaxed);
            use std::os::unix::fs::PermissionsExt;

            let test_root = std::env::var_os("HOME")
                .map(PathBuf::from)
                .expect("Linux test environment has a home directory")
                .join(".runweft-test-profiles");
            fs::create_dir_all(&test_root).expect("create local test directory");
            fs::set_permissions(&test_root, fs::Permissions::from_mode(0o700))
                .expect("make test directory private");
            let path = test_root.join(format!(
                "runweft-profile-owner-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create temporary profile directory");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .expect("make temporary profile private");
            Self(path)
        }
    }

    impl Drop for TempProfile {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn only_one_coordinator_owns_a_profile_until_its_handle_drops() {
        let profile = TempProfile::new();
        let owner = ProfileOwnership::acquire(&profile.0).expect("first owner acquires lock");

        assert!(matches!(
            ProfileOwnership::acquire(&profile.0),
            Err(OwnershipError::AlreadyOwned)
        ));

        let child = Command::new(std::env::current_exe().expect("test binary path"))
            .arg("--exact")
            .arg("ownership::tests::another_process_cannot_own_the_same_profile")
            .env("RUNWEFT_OWNERSHIP_PROBE", &profile.0)
            .status()
            .expect("start second coordinator probe");
        assert!(child.success(), "second process must observe the held lock");

        drop(owner);
        ProfileOwnership::acquire(&profile.0).expect("lock is released when owner drops");
    }

    #[test]
    fn another_process_cannot_own_the_same_profile() {
        let Ok(profile_dir) = std::env::var("RUNWEFT_OWNERSHIP_PROBE") else {
            return;
        };
        assert!(matches!(
            ProfileOwnership::acquire(profile_dir.as_ref()),
            Err(OwnershipError::AlreadyOwned)
        ));
    }

    #[test]
    fn rejects_a_profile_directory_that_is_not_private() {
        use std::os::unix::fs::PermissionsExt;

        let profile = TempProfile::new();
        fs::set_permissions(&profile.0, fs::Permissions::from_mode(0o755))
            .expect("make profile directory public");

        assert!(ProfileOwnership::acquire(&profile.0).is_err());
    }

    #[test]
    fn rejects_a_symlink_in_the_profile_path() {
        use std::os::unix::fs::symlink;

        let profile = TempProfile::new();
        let alias = profile.0.with_extension("alias");
        symlink(&profile.0, &alias).expect("create profile path symlink");

        let result = ProfileOwnership::acquire(&alias);

        fs::remove_file(alias).expect("remove profile path symlink");
        assert!(result.is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rejects_a_symlink_lock_file() {
        use std::os::unix::fs::symlink;

        let profile = TempProfile::new();
        let outside_lock = profile.0.with_extension("outside-lock");
        fs::write(&outside_lock, b"not the profile lock").expect("create outside file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&outside_lock, fs::Permissions::from_mode(0o600))
                .expect("make outside file private");
        }
        let lock_path = profile.0.join("coordinator.lock");
        symlink(&outside_lock, &lock_path).expect("create lock symlink");

        let result = ProfileOwnership::acquire(&profile.0);

        fs::remove_file(lock_path).expect("remove lock symlink");
        fs::remove_file(outside_lock).expect("remove outside file");
        assert!(result.is_err());
    }
}
