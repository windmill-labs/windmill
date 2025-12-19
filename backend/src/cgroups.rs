use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CgroupError {
    PathNotFound(PathBuf),
    NotSupported,
    PermissionDenied,
    Io(std::io::Error),
}

impl From<std::io::Error> for CgroupError {
    fn from(e: std::io::Error) -> Self {
        CgroupError::Io(e)
    }
}

pub fn get_cgroup_path() -> Result<PathBuf, CgroupError> {
    let cgroup_info = fs::read_to_string("/proc/1/cgroup")?;

    // Format: "0::/kubepods.slice/..." - we want the part after the second colon
    let cgroup_rel = cgroup_info
        .lines()
        .next()
        .and_then(|line| line.splitn(3, ':').nth(2))
        .unwrap_or("")
        .trim();

    let cgroup_path = PathBuf::from(format!("/sys/fs/cgroup{}", cgroup_rel));

    if !cgroup_path.is_dir() {
        return Err(CgroupError::PathNotFound(cgroup_path));
    }

    Ok(cgroup_path)
}

pub fn disable_oom_group() -> Result<(), CgroupError> {
    let cgroup_path = get_cgroup_path()?;
    let oom_group_file = cgroup_path.join("memory.oom.group");

    if !oom_group_file.exists() {
        return Err(CgroupError::NotSupported);
    }

    let current = fs::read_to_string(&oom_group_file)?;
    if current.trim() == "0" {
        tracing::info!("memory.oom.group already disabled");
        return Ok(());
    }

    match fs::write(&oom_group_file, "0") {
        Ok(_) => {
            tracing::info!("Disabled memory.oom.group at {:?}", cgroup_path);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            tracing::error!("Failed to disable memory.oom.group (need privileged mode)");
            Err(CgroupError::PermissionDenied)
        }
        Err(e) => Err(CgroupError::Io(e)),
    }
}
