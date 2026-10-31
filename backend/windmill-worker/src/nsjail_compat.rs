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

#[cfg(test)]
mod tests {
    // nsjail rejects a config that sets a scalar field twice, which would fail every jailed job.
    #[test]
    fn compat_config_sets_each_field_once() {
        let bun = include_str!("../nsjail/run.bun.config.proto");
        assert!(bun.contains("clone_newcgroup: false"));
        let twice = super::compat_config(&super::compat_config(bun));
        assert_eq!(twice.matches("clone_newcgroup").count(), 1);
        assert_eq!(twice.matches("gvisor_compat").count(), 1);
        assert!(twice.ends_with("\nclone_newcgroup: false\ngvisor_compat: true\n"));
    }

    /// Every nsjail profile drops its capabilities.
    #[test]
    fn every_nsjail_config_drops_capabilities() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/nsjail");
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).expect("nsjail config dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("proto") {
                continue;
            }
            let content = std::fs::read_to_string(&path).expect("read config");
            let settings = content
                .lines()
                .map(str::trim)
                .filter(|l| l.starts_with("keep_caps"))
                .collect::<Vec<_>>();
            assert_eq!(
                settings,
                vec!["keep_caps: false"],
                "{} must set keep_caps: false exactly once",
                path.display()
            );
            checked += 1;
        }
        assert!(
            checked >= 20,
            "only {checked} nsjail configs found in {dir}, expected all of them"
        );
    }

    /// `REQ` and `TARGET` reach uv as single, unaltered argv words.
    #[cfg(unix)]
    #[test]
    fn download_deps_passes_the_requirement_as_a_literal_argv_word() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("wm_download_deps_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");

        // Stand-in for uv that prints each argument it receives on its own line.
        let uv = dir.join("uv_argv_dump");
        std::fs::write(
            &uv,
            "#!/bin/sh\nfor a in \"$@\"; do printf '<%s>\\n' \"$a\"; done\n",
        )
        .expect("write stub");
        std::fs::set_permissions(&uv, std::fs::Permissions::from_mode(0o755)).expect("chmod stub");

        let script = dir.join("download_deps.sh");
        std::fs::write(
            &script,
            include_str!("../nsjail/download_deps.py.sh")
                .replace("/usr/local/bin/uv", uv.to_str().unwrap()),
        )
        .expect("write script");

        let marker = dir.join("marker");
        let req = format!("x==$(touch {})", marker.display());
        let target = dir.join("target dir with spaces");
        let out = std::process::Command::new("/bin/sh")
            .arg(&script)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("REQ", &req)
            .env("TARGET", &target)
            .output()
            .expect("run download_deps.sh");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();

        assert!(
            !marker.exists(),
            "REQ was not passed through verbatim; stdout was:\n{stdout}"
        );
        assert!(
            stdout.contains(&format!("<{req}>\n")),
            "the requirement must be one literal argv word; stdout was:\n{stdout}"
        );
        assert!(
            stdout.contains(&format!("<--target>\n<{}>\n", target.display())),
            "--target must be followed by the target dir as one argv word; stdout was:\n{stdout}"
        );

        std::fs::remove_dir_all(&dir).expect("cleanup scratch dir");
    }
}
