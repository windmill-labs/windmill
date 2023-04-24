// Adapted from https://github.com/HurricanKai/deno/blob/66a22d231af0681450b84d1bdbc0b585f0a728ee/ext/fs/std_fs.rs

use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use deno_runtime::deno_io::StdFileResource;
// use fs3::FileExt;

use deno_fs::interface::FsDirEntry;
use deno_fs::interface::FsFileType;
use deno_fs::interface::FsResult;
use deno_fs::interface::FsStat;
use deno_fs::FileSystem;
use deno_fs::OpenOptions;

#[derive(Clone)]
pub struct WindmillFs {
    inner: Arc<std::sync::Mutex<Inner>>,
}

impl WindmillFs {
    pub fn new(base_dir: PathBuf) -> Self {
        assert!(base_dir.is_absolute());
        Self { inner: Arc::new(std::sync::Mutex::new(Inner { base_dir })) }
    }

    fn inner_mut_sync(&self) -> std::sync::MutexGuard<Inner> {
        self.inner.lock().unwrap()
    }

    fn ensure_absolute_path_sync(&self, path: impl AsRef<Path>) -> PathBuf {
        self.inner_mut_sync().ensure_absolute_path_sync(path)
    }
}

struct Inner {
    base_dir: PathBuf,
}

impl Inner {
    fn ensure_absolute_path_sync(&self, path: impl AsRef<Path>) -> PathBuf {
        // TODO: as an extra line of defense, ensure an absolute path is a part of base_dir. The user should never access anything outside of base_dir.
        let mut base = self.base_dir.clone();
        base.push(path);
        base
    }
}

#[async_trait::async_trait(?Send)]
impl FileSystem for WindmillFs {
    type File = StdFileResource;

    fn cwd(&self) -> FsResult<PathBuf> {
        Ok(self.inner_mut_sync().base_dir.clone())
    }

    fn tmp_dir(&self) -> FsResult<PathBuf> {
        // TODO: Consider making a relative temp dir instead - this is sort of a leak, and deals badly with permissions
        Ok(std::env::temp_dir())
    }

    fn chdir(&self, path: impl AsRef<Path>) -> FsResult<()> {
        let mut inner = self.inner_mut_sync();
        inner.base_dir = inner.ensure_absolute_path_sync(path);
        Ok(())
    }

    #[cfg(not(unix))]
    fn umask(&self, _mask: Option<u32>) -> FsResult<u32> {
        // TODO implement umask for Windows
        // see https://github.com/nodejs/node/blob/master/src/node_process_methods.cc
        // and https://docs.microsoft.com/fr-fr/cpp/c-runtime-library/reference/umask?view=vs-2019
        Err(FsError::NotSupported)
    }

    #[cfg(unix)]
    fn umask(&self, mask: Option<u32>) -> FsResult<u32> {
        use nix::sys::stat::mode_t;
        use nix::sys::stat::umask;
        use nix::sys::stat::Mode;
        let r = if let Some(mask) = mask {
            // If mask provided, return previous.
            umask(Mode::from_bits_truncate(mask as mode_t))
        } else {
            // If no mask provided, we query the current. Requires two syscalls.
            let prev = umask(Mode::from_bits_truncate(0o777));
            let _ = umask(prev);
            prev
        };
        #[cfg(target_os = "linux")]
        {
            Ok(r.bits())
        }
        #[cfg(target_os = "macos")]
        {
            Ok(r.bits() as u32)
        }
    }

    fn open_sync(&self, path: impl AsRef<Path>, options: OpenOptions) -> FsResult<Self::File> {
        let path = self.ensure_absolute_path_sync(path);
        let opts = open_options(options);
        let std_file = opts.open(path)?;
        Ok(StdFileResource::fs_file(std_file))
    }
    async fn open_async(&self, path: PathBuf, options: OpenOptions) -> FsResult<Self::File> {
        let opts = open_options(options);
        let self = self.clone();
        let std_file = tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            opts.open(path)
        })
        .await??;
        Ok(StdFileResource::fs_file(std_file))
    }

    fn mkdir_sync(&self, path: impl AsRef<Path>, recursive: bool, mode: u32) -> FsResult<()> {
        let path = self.ensure_absolute_path_sync(path);
        mkdir(path, recursive, mode)
    }
    async fn mkdir_async(&self, path: PathBuf, recursive: bool, mode: u32) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            mkdir(path, recursive, mode)
        })
        .await?
    }

    fn chmod_sync(&self, path: impl AsRef<Path>, mode: u32) -> FsResult<()> {
        let path = self.ensure_absolute_path_sync(path);
        chmod(path, mode)
    }
    async fn chmod_async(&self, path: PathBuf, mode: u32) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            chmod(path, mode)
        })
        .await?
    }

    fn chown_sync(
        &self,
        path: impl AsRef<Path>,
        uid: Option<u32>,
        gid: Option<u32>,
    ) -> FsResult<()> {
        let path = self.ensure_absolute_path_sync(path);
        chown(path, uid, gid)
    }
    async fn chown_async(&self, path: PathBuf, uid: Option<u32>, gid: Option<u32>) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            chown(path, uid, gid)
        })
        .await?
    }

    fn remove_sync(&self, path: impl AsRef<Path>, recursive: bool) -> FsResult<()> {
        let path = self.ensure_absolute_path_sync(path);
        remove(path, recursive)
    }
    async fn remove_async(&self, path: PathBuf, recursive: bool) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            remove(path, recursive)
        })
        .await?
    }

    fn copy_file_sync(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> FsResult<()> {
        let from = self.ensure_absolute_path_sync(from);
        let to = self.ensure_absolute_path_sync(to);

        copy_file(from, to)
    }
    async fn copy_file_async(&self, from: PathBuf, to: PathBuf) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let from = self.ensure_absolute_path_sync(from);
            let to = self.ensure_absolute_path_sync(to);
            copy_file(from, to)
        })
        .await?
    }

    fn stat_sync(&self, path: impl AsRef<Path>) -> FsResult<FsStat> {
        let path = self.ensure_absolute_path_sync(path);
        stat(path).map(Into::into)
    }
    async fn stat_async(&self, path: PathBuf) -> FsResult<FsStat> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            stat(path)
        })
        .await?
        .map(Into::into)
    }

    fn lstat_sync(&self, path: impl AsRef<Path>) -> FsResult<FsStat> {
        let path = self.ensure_absolute_path_sync(path);
        lstat(path).map(Into::into)
    }
    async fn lstat_async(&self, path: PathBuf) -> FsResult<FsStat> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            lstat(path)
        })
        .await?
        .map(Into::into)
    }

    fn realpath_sync(&self, path: impl AsRef<Path>) -> FsResult<PathBuf> {
        let path = self.ensure_absolute_path_sync(path);
        realpath(path)
    }
    async fn realpath_async(&self, path: PathBuf) -> FsResult<PathBuf> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            realpath(path)
        })
        .await?
    }

    fn read_dir_sync(&self, path: impl AsRef<Path>) -> FsResult<Vec<FsDirEntry>> {
        let path = self.ensure_absolute_path_sync(path);
        read_dir(path)
    }
    async fn read_dir_async(&self, path: PathBuf) -> FsResult<Vec<FsDirEntry>> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            read_dir(path)
        })
        .await?
    }

    fn rename_sync(&self, oldpath: impl AsRef<Path>, newpath: impl AsRef<Path>) -> FsResult<()> {
        let oldpath = self.ensure_absolute_path_sync(oldpath);
        let newpath = self.ensure_absolute_path_sync(newpath);
        fs::rename(oldpath, newpath).map_err(Into::into)
    }
    async fn rename_async(&self, oldpath: PathBuf, newpath: PathBuf) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let oldpath = self.ensure_absolute_path_sync(oldpath);
            let newpath = self.ensure_absolute_path_sync(newpath);
            fs::rename(oldpath, newpath)
        })
        .await?
        .map_err(Into::into)
    }

    fn link_sync(&self, oldpath: impl AsRef<Path>, newpath: impl AsRef<Path>) -> FsResult<()> {
        let oldpath = self.ensure_absolute_path_sync(oldpath);
        let newpath = self.ensure_absolute_path_sync(newpath);
        fs::hard_link(oldpath, newpath).map_err(Into::into)
    }
    async fn link_async(&self, oldpath: PathBuf, newpath: PathBuf) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let oldpath = self.ensure_absolute_path_sync(oldpath);
            let newpath = self.ensure_absolute_path_sync(newpath);
            fs::hard_link(oldpath, newpath)
        })
        .await?
        .map_err(Into::into)
    }

    fn symlink_sync(
        &self,
        oldpath: impl AsRef<Path>,
        newpath: impl AsRef<Path>,
        file_type: Option<FsFileType>,
    ) -> FsResult<()> {
        let oldpath = self.ensure_absolute_path_sync(oldpath);
        let newpath = self.ensure_absolute_path_sync(newpath);
        symlink(oldpath, newpath, file_type)
    }
    async fn symlink_async(
        &self,
        oldpath: PathBuf,
        newpath: PathBuf,
        file_type: Option<FsFileType>,
    ) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let oldpath = self.ensure_absolute_path_sync(oldpath);
            let newpath = self.ensure_absolute_path_sync(newpath);
            symlink(oldpath, newpath, file_type)
        })
        .await?
    }

    fn read_link_sync(&self, path: impl AsRef<Path>) -> FsResult<PathBuf> {
        let path = self.ensure_absolute_path_sync(path);
        fs::read_link(path).map_err(Into::into)
    }
    async fn read_link_async(&self, path: PathBuf) -> FsResult<PathBuf> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            fs::read_link(path)
        })
        .await?
        .map_err(Into::into)
    }

    fn truncate_sync(&self, path: impl AsRef<Path>, len: u64) -> FsResult<()> {
        let path = self.ensure_absolute_path_sync(path);
        truncate(path, len)
    }
    async fn truncate_async(&self, path: PathBuf, len: u64) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            truncate(path, len)
        })
        .await?
    }

    fn utime_sync(
        &self,
        path: impl AsRef<Path>,
        atime_secs: i64,
        atime_nanos: u32,
        mtime_secs: i64,
        mtime_nanos: u32,
    ) -> FsResult<()> {
        let path = self.ensure_absolute_path_sync(path);
        let atime = filetime::FileTime::from_unix_time(atime_secs, atime_nanos);
        let mtime = filetime::FileTime::from_unix_time(mtime_secs, mtime_nanos);
        filetime::set_file_times(path, atime, mtime).map_err(Into::into)
    }
    async fn utime_async(
        &self,
        path: PathBuf,
        atime_secs: i64,
        atime_nanos: u32,
        mtime_secs: i64,
        mtime_nanos: u32,
    ) -> FsResult<()> {
        let atime = filetime::FileTime::from_unix_time(atime_secs, atime_nanos);
        let mtime = filetime::FileTime::from_unix_time(mtime_secs, mtime_nanos);
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            filetime::set_file_times(path, atime, mtime).map_err(Into::into)
        })
        .await?
    }

    fn write_file_sync(
        &self,
        path: impl AsRef<Path>,
        options: OpenOptions,
        data: &[u8],
    ) -> FsResult<()> {
        let path = self.ensure_absolute_path_sync(path);
        let opts = open_options(options);
        let mut file = opts.open(path)?;
        #[cfg(unix)]
        if let Some(mode) = options.mode {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(mode))?;
        }
        file.write_all(data)?;
        Ok(())
    }

    async fn write_file_async(
        &self,
        path: PathBuf,
        options: OpenOptions,
        data: Vec<u8>,
    ) -> FsResult<()> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            let opts = open_options(options);
            let mut file = opts.open(path)?;
            #[cfg(unix)]
            if let Some(mode) = options.mode {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(mode))?;
            }
            file.write_all(&data)?;
            Ok(())
        })
        .await?
    }

    fn read_file_sync(&self, path: impl AsRef<Path>) -> FsResult<Vec<u8>> {
        let path = self.ensure_absolute_path_sync(path);
        fs::read(path).map_err(Into::into)
    }
    async fn read_file_async(&self, path: PathBuf) -> FsResult<Vec<u8>> {
        let self = self.clone();
        tokio::task::spawn_blocking(move || {
            let path = self.ensure_absolute_path_sync(path);
            fs::read(path)
        })
        .await?
        .map_err(Into::into)
    }
}

fn mkdir(path: impl AsRef<Path>, recursive: bool, mode: u32) -> FsResult<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(recursive);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(mode);
    }
    #[cfg(not(unix))]
    {
        _ = mode;
    }
    builder.create(path).map_err(Into::into)
}

#[cfg(unix)]
fn chmod(path: impl AsRef<Path>, mode: u32) -> FsResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = fs::Permissions::from_mode(mode);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

// TODO: implement chmod for Windows (#4357)
#[cfg(not(unix))]
fn chmod(path: impl AsRef<Path>, _mode: u32) -> FsResult<()> {
    // Still check file/dir exists on Windows
    std::fs::metadata(path)?;
    Err(FsError::NotSupported)
}

#[cfg(unix)]
fn chown(path: impl AsRef<Path>, uid: Option<u32>, gid: Option<u32>) -> FsResult<()> {
    use nix::unistd::chown;
    use nix::unistd::Gid;
    use nix::unistd::Uid;
    let owner = uid.map(Uid::from_raw);
    let group = gid.map(Gid::from_raw);
    let res = chown(path.as_ref(), owner, group);
    if let Err(err) = res {
        return Err(io::Error::from_raw_os_error(err as i32).into());
    }
    Ok(())
}

// TODO: implement chown for Windows
#[cfg(not(unix))]
fn chown(_path: impl AsRef<Path>, _uid: Option<u32>, _gid: Option<u32>) -> FsResult<()> {
    Err(FsError::NotSupported)
}

fn remove(path: impl AsRef<Path>, recursive: bool) -> FsResult<()> {
    // TODO: this is racy. This should open fds, and then `unlink` those.
    let metadata = fs::symlink_metadata(&path)?;

    let file_type = metadata.file_type();
    let res = if file_type.is_dir() {
        if recursive {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_dir(&path)
        }
    } else if file_type.is_symlink() {
        #[cfg(unix)]
        {
            fs::remove_file(&path)
        }
        #[cfg(not(unix))]
        {
            use std::os::windows::prelude::MetadataExt;
            use winapi::um::winnt::FILE_ATTRIBUTE_DIRECTORY;
            if metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0 {
                fs::remove_dir(&path)
            } else {
                fs::remove_file(&path)
            }
        }
    } else {
        fs::remove_file(&path)
    };

    res.map_err(Into::into)
}

fn copy_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> FsResult<()> {
    #[cfg(target_os = "macos")]
    {
        use libc::clonefile;
        use libc::stat;
        use libc::unlink;
        use std::ffi::CString;
        use std::os::unix::fs::OpenOptionsExt;
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::prelude::OsStrExt;

        let from_str = CString::new(from.as_ref().as_os_str().as_bytes()).unwrap();
        let to_str = CString::new(to.as_ref().as_os_str().as_bytes()).unwrap();

        // SAFETY: `from` and `to` are valid C strings.
        // std::fs::copy does open() + fcopyfile() on macOS. We try to use
        // clonefile() instead, which is more efficient.
        unsafe {
            let mut st = std::mem::zeroed();
            let ret = stat(from_str.as_ptr(), &mut st);
            if ret != 0 {
                return Err(io::Error::last_os_error().into());
            }

            if st.st_size > 128 * 1024 {
                // Try unlink. If it fails, we are going to try clonefile() anyway.
                let _ = unlink(to_str.as_ptr());
                // Matches rust stdlib behavior for io::copy.
                // https://github.com/rust-lang/rust/blob/3fdd578d72a24d4efc2fe2ad18eec3b6ba72271e/library/std/src/sys/unix/fs.rs#L1613-L1616
                if clonefile(from_str.as_ptr(), to_str.as_ptr(), 0) == 0 {
                    return Ok(());
                }
            } else {
                // Do a regular copy. fcopyfile() is an overkill for < 128KB
                // files.
                let mut buf = [0u8; 128 * 1024];
                let mut from_file = fs::File::open(&from)?;
                let perm = from_file.metadata()?.permissions();

                let mut to_file = fs::OpenOptions::new()
                    // create the file with the correct mode right away
                    .mode(perm.mode())
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&to)?;
                let writer_metadata = to_file.metadata()?;
                if writer_metadata.is_file() {
                    // Set the correct file permissions, in case the file already existed.
                    // Don't set the permissions on already existing non-files like
                    // pipes/FIFOs or device nodes.
                    to_file.set_permissions(perm)?;
                }
                loop {
                    let nread = from_file.read(&mut buf)?;
                    if nread == 0 {
                        break;
                    }
                    to_file.write_all(&buf[..nread])?;
                }
                return Ok(());
            }
        }

        // clonefile() failed, fall back to std::fs::copy().
    }

    fs::copy(from, to)?;

    Ok(())
}

#[cfg(not(windows))]
fn stat(path: impl AsRef<Path>) -> FsResult<FsStat> {
    let metadata = fs::metadata(path)?;
    Ok(metadata_to_fsstat(metadata))
}

#[cfg(windows)]
fn stat(path: impl AsRef<Path>) -> FsResult<FsStat> {
    let metadata = fs::metadata(path.as_ref())?;
    let mut fsstat = metadata_to_fsstat(metadata);
    use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;
    let path = path.as_ref().canonicalize()?;
    stat_extra(&mut fsstat, &path, FILE_FLAG_BACKUP_SEMANTICS)?;
    Ok(fsstat)
}

#[cfg(not(windows))]
fn lstat(path: impl AsRef<Path>) -> FsResult<FsStat> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(metadata_to_fsstat(metadata))
}

#[cfg(windows)]
fn lstat(path: impl AsRef<Path>) -> FsResult<FsStat> {
    let metadata = fs::symlink_metadata(path.as_ref())?;
    let mut fsstat = metadata_to_fsstat(metadata);
    use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;
    use winapi::um::winbase::FILE_FLAG_OPEN_REPARSE_POINT;
    stat_extra(
        &mut fsstat,
        path.as_ref(),
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
    )?;
    Ok(fsstat)
}

#[cfg(windows)]
fn stat_extra(
    fsstat: &mut FsStat,
    path: &Path,
    file_flags: winapi::shared::minwindef::DWORD,
) -> FsResult<()> {
    use std::os::windows::prelude::OsStrExt;

    use winapi::um::fileapi::CreateFileW;
    use winapi::um::fileapi::OPEN_EXISTING;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::winnt::FILE_SHARE_DELETE;
    use winapi::um::winnt::FILE_SHARE_READ;
    use winapi::um::winnt::FILE_SHARE_WRITE;

    unsafe fn get_dev(handle: winapi::shared::ntdef::HANDLE) -> std::io::Result<u64> {
        use winapi::shared::minwindef::FALSE;
        use winapi::um::fileapi::GetFileInformationByHandle;
        use winapi::um::fileapi::BY_HANDLE_FILE_INFORMATION;

        let info = {
            let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
            if GetFileInformationByHandle(handle, info.as_mut_ptr()) == FALSE {
                return Err(std::io::Error::last_os_error());
            }

            info.assume_init()
        };

        Ok(info.dwVolumeSerialNumber as u64)
    }

    // SAFETY: winapi calls
    unsafe {
        let mut path: Vec<_> = path.as_os_str().encode_wide().collect();
        path.push(0);
        let file_handle = CreateFileW(
            path.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_DELETE | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            file_flags,
            std::ptr::null_mut(),
        );
        if file_handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error().into());
        }

        let result = get_dev(file_handle);
        CloseHandle(file_handle);
        fsstat.dev = result?;

        Ok(())
    }
}

#[inline(always)]
fn metadata_to_fsstat(metadata: fs::Metadata) -> FsStat {
    macro_rules! unix_or_zero {
        ($member:ident) => {{
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                metadata.$member()
            }
            #[cfg(not(unix))]
            {
                0
            }
        }};
    }

    #[inline(always)]
    fn to_msec(maybe_time: Result<SystemTime, io::Error>) -> Option<u64> {
        match maybe_time {
            Ok(time) => Some(
                time.duration_since(UNIX_EPOCH)
                    .map(|t| t.as_millis() as u64)
                    .unwrap_or_else(|err| err.duration().as_millis() as u64),
            ),
            Err(_) => None,
        }
    }

    FsStat {
        is_file: metadata.is_file(),
        is_directory: metadata.is_dir(),
        is_symlink: metadata.file_type().is_symlink(),
        size: metadata.len(),

        mtime: to_msec(metadata.modified()),
        atime: to_msec(metadata.accessed()),
        birthtime: to_msec(metadata.created()),

        dev: unix_or_zero!(dev),
        ino: unix_or_zero!(ino),
        mode: unix_or_zero!(mode),
        nlink: unix_or_zero!(nlink),
        uid: unix_or_zero!(uid),
        gid: unix_or_zero!(gid),
        rdev: unix_or_zero!(rdev),
        blksize: unix_or_zero!(blksize),
        blocks: unix_or_zero!(blocks),
    }
}

fn realpath(path: impl AsRef<Path>) -> FsResult<PathBuf> {
    let canonicalized_path = path.as_ref().canonicalize()?;
    #[cfg(windows)]
    let canonicalized_path = PathBuf::from(
        canonicalized_path
            .display()
            .to_string()
            .trim_start_matches("\\\\?\\"),
    );
    Ok(canonicalized_path)
}

fn read_dir(path: impl AsRef<Path>) -> FsResult<Vec<FsDirEntry>> {
    let entries = fs::read_dir(path)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;
            let metadata = entry.file_type();
            macro_rules! method_or_false {
                ($method:ident) => {
                    if let Ok(metadata) = &metadata {
                        metadata.$method()
                    } else {
                        false
                    }
                };
            }
            Some(FsDirEntry {
                name,
                is_file: method_or_false!(is_file),
                is_directory: method_or_false!(is_dir),
                is_symlink: method_or_false!(is_symlink),
            })
        })
        .collect();

    Ok(entries)
}

#[cfg(not(windows))]
fn symlink(
    oldpath: impl AsRef<Path>,
    newpath: impl AsRef<Path>,
    _file_type: Option<FsFileType>,
) -> FsResult<()> {
    std::os::unix::fs::symlink(oldpath.as_ref(), newpath.as_ref())?;
    Ok(())
}

#[cfg(windows)]
fn symlink(
    oldpath: impl AsRef<Path>,
    newpath: impl AsRef<Path>,
    file_type: Option<FsFileType>,
) -> FsResult<()> {
    let file_type = match file_type {
        Some(file_type) => file_type,
        None => {
            let old_meta = fs::metadata(&oldpath);
            match old_meta {
                Ok(metadata) => {
                    if metadata.is_file() {
                        FsFileType::File
                    } else if metadata.is_dir() {
                        FsFileType::Directory
                    } else {
                        return Err(FsError::Io(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "On Windows the target must be a file or directory",
                        )));
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    return Err(FsError::Io(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "On Windows an `options` argument is required if the target does not exist",
                    )))
                }
                Err(err) => return Err(err.into()),
            }
        }
    };

    match file_type {
        FsFileType::File => {
            std::os::windows::fs::symlink_file(&oldpath, &newpath)?;
        }
        FsFileType::Directory => {
            std::os::windows::fs::symlink_dir(&oldpath, &newpath)?;
        }
    };

    Ok(())
}

fn truncate(path: impl AsRef<Path>, len: u64) -> FsResult<()> {
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_len(len)?;
    Ok(())
}

fn open_options(options: OpenOptions) -> fs::OpenOptions {
    let mut open_options = fs::OpenOptions::new();
    if let Some(mode) = options.mode {
        // mode only used if creating the file on Unix
        // if not specified, defaults to 0o666
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            open_options.mode(mode & 0o777);
        }
        #[cfg(not(unix))]
        let _ = mode; // avoid unused warning
    }
    open_options.read(options.read);
    open_options.create(options.create);
    open_options.write(options.write);
    open_options.truncate(options.truncate);
    open_options.append(options.append);
    open_options.create_new(options.create_new);
    open_options
}
