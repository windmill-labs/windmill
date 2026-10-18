//! `NSJAIL_GVISOR_COMPAT=true`: run nsjail inside gVisor (runsc). Each config is rewritten
//! right before nsjail starts, with `gvisor_compat: true` (an option of the nsjail the images
//! build, from `docker/nsjail-patches/`) and no cgroup namespace, which gVisor refuses.

use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::Command;

pub(crate) fn compat_config(config: &str) -> String {
    let mut out = config
        .lines()
        .filter(|l| {
            let l = l.trim_start();
            !l.starts_with("clone_newcgroup") && !l.starts_with("gvisor_compat")
        })
        .collect::<Vec<_>>()
        .join("\n");
    out.push_str("\nclone_newcgroup: false\ngvisor_compat: true\n");
    out
}

/// Rewrites the config `cmd` (an nsjail command line) passes with `--config`.
pub(crate) fn apply(cmd: &Command) -> io::Result<()> {
    let args = cmd.get_args().collect::<Vec<&OsStr>>();
    let Some(config) = args.windows(2).find(|w| w[0] == "--config").map(|w| w[1]) else {
        return Ok(());
    };
    let path = match cmd.get_current_dir() {
        Some(dir) => dir.join(config),
        None => PathBuf::from(config),
    };
    let rewritten = compat_config(&std::fs::read_to_string(&path)?);
    std::fs::write(&path, rewritten)
}
