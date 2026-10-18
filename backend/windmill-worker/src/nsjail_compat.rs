//! `NSJAIL_GVISOR_COMPAT=true`: run nsjail inside gVisor (runsc), which refuses
//! `clone(CLONE_NEWCGROUP)` and `setrlimit(RLIMIT_MSGQUEUE)`, both of which nsjail does by
//! default. Each config is rewritten right before nsjail starts: no cgroup namespace, and
//! `disable_rl` with the config's own rlimits set on the nsjail process, which the jail
//! inherits. A limit already at its value is not set again, so the inherited message
//! queue limit, which nsjail only reasserts, never reaches gVisor.

use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use nix::libc;

/// Marks a config whose `disable_rl` was added here, so that a config spawned twice keeps its
/// limits: its `rlimit_*` lines are still in it, and nsjail ignores them under `disable_rl`.
const COMPAT_MARKER: &str = "# NSJAIL_GVISOR_COMPAT: the worker sets the rlimits below";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RlimitValue {
    Value(u64),
    Soft,
    Hard,
    Inf,
}

/// nsjail's rlimit options: name, resource, default, default type and unit (nsjail
/// config.proto). `Value(0)` stands for the type VALUE, whose value is `rlimit_<name>`.
#[rustfmt::skip]
const RLIMITS: [(&str, Resource, u64, RlimitValue, u64); 10] = [
    ("as",       libc::RLIMIT_AS,       4096, RlimitValue::Value(0), 1 << 20),
    ("core",     libc::RLIMIT_CORE,     0,    RlimitValue::Value(0), 1 << 20),
    ("cpu",      libc::RLIMIT_CPU,      600,  RlimitValue::Value(0), 1),
    ("fsize",    libc::RLIMIT_FSIZE,    1,    RlimitValue::Value(0), 1 << 20),
    ("nofile",   libc::RLIMIT_NOFILE,   32,   RlimitValue::Value(0), 1),
    ("nproc",    libc::RLIMIT_NPROC,    1024, RlimitValue::Soft,     1),
    ("stack",    libc::RLIMIT_STACK,    8,    RlimitValue::Soft,     1 << 20),
    ("memlock",  libc::RLIMIT_MEMLOCK,  64,   RlimitValue::Soft,     1 << 10),
    ("rtprio",   libc::RLIMIT_RTPRIO,   0,    RlimitValue::Soft,     1),
    ("msgqueue", libc::RLIMIT_MSGQUEUE, 1024, RlimitValue::Soft,     1),
];

#[cfg(target_env = "gnu")]
type Resource = libc::__rlimit_resource_t;
#[cfg(not(target_env = "gnu"))]
type Resource = libc::c_int;

/// The config nsjail runs under gVisor, and the rlimits it would have set itself, or `None`
/// when the config already leaves them to the parent.
pub(crate) fn compat_config(config: &str) -> (String, Option<Vec<(Resource, RlimitValue)>>) {
    let field = |name: &str| {
        config.lines().rev().find_map(|l| {
            let v = l
                .trim()
                .strip_prefix(name)?
                .trim_start()
                .strip_prefix(':')?;
            v.split('#').next().map(str::trim)
        })
    };
    let marked = config.lines().any(|l| l.trim() == COMPAT_MARKER);
    let rlimits = (marked || field("disable_rl") != Some("true")).then(|| {
        RLIMITS
            .iter()
            .map(|&(name, res, default, default_type, unit)| {
                let value = field(&format!("rlimit_{name}"))
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(default);
                let kind = match field(&format!("rlimit_{name}_type")) {
                    Some("SOFT") => RlimitValue::Soft,
                    Some("HARD") => RlimitValue::Hard,
                    Some("INF") => RlimitValue::Inf,
                    Some("VALUE") => RlimitValue::Value(0),
                    _ => default_type,
                };
                let kind = match kind {
                    RlimitValue::Value(_) => RlimitValue::Value(value.saturating_mul(unit)),
                    k => k,
                };
                (res, kind)
            })
            .collect()
    });

    let mut out = config
        .lines()
        .filter(|l| !l.trim_start().starts_with("clone_newcgroup"))
        .collect::<Vec<_>>()
        .join("\n");
    out.push_str("\nclone_newcgroup: false\n");
    if rlimits.is_some() && !marked {
        out.push_str(COMPAT_MARKER);
        out.push_str("\ndisable_rl: true\n");
    }
    (out, rlimits)
}

/// Rewrites the config `cmd` (an nsjail command line) passes with `--config`, and makes the
/// child set that config's rlimits on itself before it execs nsjail.
pub(crate) fn apply(cmd: &mut Command) -> io::Result<()> {
    let args = cmd.get_args().collect::<Vec<&OsStr>>();
    let Some(config) = args.windows(2).find(|w| w[0] == "--config").map(|w| w[1]) else {
        return Ok(());
    };
    let path = match cmd.get_current_dir() {
        Some(dir) => dir.join(config),
        None => PathBuf::from(config),
    };
    let (rewritten, rlimits) = compat_config(&std::fs::read_to_string(&path)?);
    std::fs::write(&path, rewritten)?;
    let Some(rlimits) = rlimits else {
        return Ok(());
    };
    let limits = rlimits
        .into_iter()
        .map(|(res, value)| {
            let current = getrlimit(res)?;
            let v = match value {
                RlimitValue::Value(v) => v,
                RlimitValue::Soft => current.rlim_cur,
                RlimitValue::Hard => current.rlim_max,
                RlimitValue::Inf => libc::RLIM_INFINITY,
            };
            Ok((res, v))
        })
        .collect::<io::Result<Vec<_>>>()?;
    use std::os::unix::process::CommandExt;
    // Only raw syscalls here, between fork and exec.
    unsafe {
        cmd.pre_exec(move || {
            for &(res, v) in &limits {
                let current = getrlimit(res)?;
                if current.rlim_cur == v && current.rlim_max == v {
                    continue;
                }
                let new = libc::rlimit { rlim_cur: v, rlim_max: v };
                if libc::setrlimit(res, &new) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    Ok(())
}

fn getrlimit(res: Resource) -> io::Result<libc::rlimit> {
    let mut current = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    if unsafe { libc::getrlimit(res, &mut current) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    // What nsjail itself would set for the python run profile, and the same again for a
    // config the compat rewrite already went through.
    #[test]
    fn compat_config_keeps_the_profile_rlimits() {
        let profile = include_str!("../nsjail/run.python3.config.proto")
            .replace("{RLIMIT_AS}", "rlimit_as: 4096");
        let (rewritten, rlimits) = compat_config(&profile);
        let mib = 1 << 20;
        let expected = vec![
            (libc::RLIMIT_AS, RlimitValue::Value(4096 * mib)),
            (libc::RLIMIT_CORE, RlimitValue::Value(0)),
            (libc::RLIMIT_CPU, RlimitValue::Value(1000)),
            (libc::RLIMIT_FSIZE, RlimitValue::Value(1000 * mib)),
            (libc::RLIMIT_NOFILE, RlimitValue::Value(10000)),
            (libc::RLIMIT_NPROC, RlimitValue::Soft),
            (libc::RLIMIT_STACK, RlimitValue::Soft),
            (libc::RLIMIT_MEMLOCK, RlimitValue::Soft),
            (libc::RLIMIT_RTPRIO, RlimitValue::Soft),
            (libc::RLIMIT_MSGQUEUE, RlimitValue::Soft),
        ];
        assert_eq!(rlimits.as_ref(), Some(&expected));
        assert!(rewritten.contains("\nclone_newcgroup: false\n"));
        assert!(rewritten.contains("\ndisable_rl: true\n"));

        let (again, rlimits) = compat_config(&rewritten);
        assert_eq!(rlimits, Some(expected));
        assert_eq!(again.matches("disable_rl").count(), 1);
        assert_eq!(again.matches("clone_newcgroup").count(), 1);

        let (_, rlimits) = compat_config(include_str!("../nsjail/run.bash.config.proto"));
        assert_eq!(rlimits, None);
    }
}
