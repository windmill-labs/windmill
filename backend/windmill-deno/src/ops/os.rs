// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use crate::windmill_worker::ExitCode;
use deno_core::error::type_error;
use deno_core::error::AnyError;
use deno_core::op;
use deno_core::v8;
use deno_core::OpState;
use deno_runtime::permissions::PermissionsContainer;
use serde::Serialize;
use std::collections::HashMap;

struct EnvVars(HashMap<String, String>);

impl EnvVars {
    pub fn set(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }

    pub fn all(&self) -> HashMap<String, String> {
        self.0.clone()
    }
}

deno_core::ops!(
    deno_ops,
    [
        op_env,
        op_exec_path,
        op_exit,
        op_delete_env,
        op_get_env,
        op_gid,
        op_hostname,
        op_loadavg,
        op_network_interfaces,
        op_os_release,
        op_os_uptime,
        op_node_unstable_os_uptime,
        op_set_env,
        op_set_exit_code,
        op_system_memory_info,
        op_uid,
        op_runtime_memory_usage,
    ]
);

deno_core::extension!(
  deno_os,
  ops_fn = deno_ops,
  options = {
    exit_code: ExitCode,
    env_vars: HashMap<String, String>,
  },
  state = |state, options| {
    state.put::<ExitCode>(options.exit_code);
    state.put::<EnvVars>(EnvVars(options.env_vars));
  },
  customizer = |ext: &mut deno_core::ExtensionBuilder| {
    ext.force_op_registration();
  }
);

deno_core::extension!(
    deno_os_worker,
    ops_fn = deno_ops,
    middleware = |op| match op.name {
        "op_exit" | "op_set_exit_code" => op.disable(),
        _ => op,
    },
    customizer = |ext: &mut deno_core::ExtensionBuilder| {
        ext.force_op_registration();
    }
);

#[op]
fn op_exec_path(state: &mut OpState) -> Result<String, AnyError> {
    state
        .borrow_mut::<PermissionsContainer>()
        .check_read_blind(
            std::path::Path::new("/exec_path"),
            "exec_path",
            "Deno.execPath()",
        )?;

    unreachable!("exec_path permission check should always reject")
}

#[op]
fn op_set_env(state: &mut OpState, key: &str, value: &str) -> Result<(), AnyError> {
    state.borrow_mut::<PermissionsContainer>().check_env(key)?;
    if key.is_empty() {
        return Err(type_error("Key is an empty string."));
    }
    if key.contains(&['=', '\0'] as &[char]) {
        return Err(type_error(format!(
            "Key contains invalid characters: {key:?}"
        )));
    }
    if value.contains('\0') {
        return Err(type_error(format!(
            "Value contains invalid characters: {value:?}"
        )));
    }

    state
        .borrow_mut::<EnvVars>()
        .set(key.to_owned(), value.to_owned());
    Ok(())
}

#[op]
fn op_env(state: &mut OpState) -> Result<HashMap<String, String>, AnyError> {
    state.borrow_mut::<PermissionsContainer>().check_env_all()?;
    Ok(state.borrow::<EnvVars>().all())
}

#[op]
fn op_get_env(state: &mut OpState, key: String) -> Result<Option<String>, AnyError> {
    state.borrow_mut::<PermissionsContainer>().check_env(&key)?;

    if key.is_empty() {
        return Err(type_error("Key is an empty string."));
    }

    if key.contains(&['=', '\0'] as &[char]) {
        return Err(type_error(format!(
            "Key contains invalid characters: {key:?}"
        )));
    }

    Ok(state.borrow::<EnvVars>().get(&key).map(|e| e.to_owned()))
}

#[op]
fn op_delete_env(state: &mut OpState, key: String) -> Result<(), AnyError> {
    state.borrow_mut::<PermissionsContainer>().check_env(&key)?;
    if key.is_empty() || key.contains(&['=', '\0'] as &[char]) {
        return Err(type_error("Key contains invalid characters."));
    }
    state.borrow_mut::<EnvVars>().remove(&key);
    Ok(())
}

#[op]
fn op_set_exit_code(state: &mut OpState, code: i32) {
    state.borrow_mut::<ExitCode>().set(code);
}

#[op]
fn op_exit(state: &mut OpState) {
    let code = state.borrow::<ExitCode>().get();
    std::process::exit(code)
}

#[op]
fn op_loadavg(state: &mut OpState) -> Result<(f64, f64, f64), AnyError> {
    state
        .borrow_mut::<PermissionsContainer>()
        .check_sys("loadavg", "Deno.loadavg()")?;
    unreachable!("loadavg permission check should always reject")
}

#[op]
fn op_hostname(state: &mut OpState) -> Result<String, AnyError> {
    state
        .borrow_mut::<PermissionsContainer>()
        .check_sys("hostname", "Deno.hostname()")?;
    unreachable!("hostname permission check should always reject")
}

#[op]
fn op_os_release(state: &mut OpState) -> Result<String, AnyError> {
    state
        .borrow_mut::<PermissionsContainer>()
        .check_sys("osRelease", "Deno.osRelease()")?;
    unreachable!("osRelease permission check should always reject")
}

#[op]
fn op_network_interfaces(
    state: &mut OpState,
) -> Result<Vec<() /* really ! but that is unstable */>, AnyError> {
    state
        .borrow_mut::<PermissionsContainer>()
        .check_sys("networkInterfaces", "Deno.networkInterfaces()")?;
    unreachable!("networkInterfaces permission check should always reject")
}

#[op]
fn op_system_memory_info(
    state: &mut OpState,
) -> Result<Option<() /* really ! but that is unstable */>, AnyError> {
    state
        .borrow_mut::<PermissionsContainer>()
        .check_sys("systemMemoryInfo", "Deno.systemMemoryInfo()")?;
    unreachable!("systemMemoryInfo permission check should always reject")
}

#[op]
fn op_gid(state: &mut OpState) -> Result<Option<u32>, AnyError> {
    state
        .borrow_mut::<PermissionsContainer>()
        .check_sys("gid", "Deno.gid()")?;

    unreachable!("gid permission check should always reject")
}

#[op]
fn op_uid(state: &mut OpState) -> Result<Option<u32>, AnyError> {
    state
        .borrow_mut::<PermissionsContainer>()
        .check_sys("uid", "Deno.uid()")?;
    unreachable!("uid permission check should always reject")
}
// HeapStats stores values from a isolate.get_heap_statistics() call
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryUsage {
    rss: usize,
    heap_total: usize,
    heap_used: usize,
    external: usize,
}

#[op(v8)]
fn op_runtime_memory_usage(scope: &mut v8::HandleScope) -> MemoryUsage {
    let mut s = v8::HeapStatistics::default();
    scope.get_heap_statistics(&mut s);
    MemoryUsage {
        rss: rss(),
        heap_total: s.total_heap_size(),
        heap_used: s.used_heap_size(),
        external: s.external_memory(),
    }
}

#[cfg(target_os = "linux")]
fn rss() -> usize {
    // Inspired by https://github.com/Arc-blroth/memory-stats/blob/5364d0d09143de2a470d33161b2330914228fde9/src/linux.rs

    // Extracts a positive integer from a string that
    // may contain leading spaces and trailing chars.
    // Returns the extracted number and the index of
    // the next character in the string.
    fn scan_int(string: &str) -> (usize, usize) {
        let mut out = 0;
        let mut idx = 0;
        let mut chars = string.chars().peekable();
        while let Some(' ') = chars.next_if_eq(&' ') {
            idx += 1;
        }
        for n in chars {
            idx += 1;
            if n.is_ascii_digit() {
                out *= 10;
                out += n as usize - '0' as usize;
            } else {
                break;
            }
        }
        (out, idx)
    }

    let statm_content = if let Ok(c) = std::fs::read_to_string("/proc/self/statm") {
        c
    } else {
        return 0;
    };

    // statm returns the virtual size and rss, in
    // multiples of the page size, as the first
    // two columns of output.
    // SAFETY: libc call
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };

    if page_size < 0 {
        return 0;
    }

    let (_total_size_pages, idx) = scan_int(&statm_content);
    let (total_rss_pages, _) = scan_int(&statm_content[idx..]);

    total_rss_pages * page_size as usize
}

#[cfg(target_os = "macos")]
fn rss() -> usize {
    // Inspired by https://github.com/Arc-blroth/memory-stats/blob/5364d0d09143de2a470d33161b2330914228fde9/src/darwin.rs

    let mut task_info = std::mem::MaybeUninit::<libc::mach_task_basic_info_data_t>::uninit();
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    // SAFETY: libc calls
    let r = unsafe {
        libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            task_info.as_mut_ptr() as libc::task_info_t,
            &mut count as *mut libc::mach_msg_type_number_t,
        )
    };
    // According to libuv this should never fail
    assert_eq!(r, libc::KERN_SUCCESS);
    // SAFETY: we just asserted that it was success
    let task_info = unsafe { task_info.assume_init() };
    task_info.resident_size as usize
}

#[cfg(windows)]
fn rss() -> usize {
    use winapi::shared::minwindef::DWORD;
    use winapi::shared::minwindef::FALSE;
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::psapi::GetProcessMemoryInfo;
    use winapi::um::psapi::PROCESS_MEMORY_COUNTERS;

    // SAFETY: winapi calls
    unsafe {
        // this handle is a constant—no need to close it
        let current_process = GetCurrentProcess();
        let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();

        if GetProcessMemoryInfo(
            current_process,
            &mut pmc,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as DWORD,
        ) != FALSE
        {
            pmc.WorkingSetSize
        } else {
            0
        }
    }
}

fn os_uptime(state: &mut OpState) -> Result<u64, AnyError> {
    state
        .borrow_mut::<PermissionsContainer>()
        .check_sys("osUptime", "Deno.osUptime()")?;

    // TODO: Maybe allow this for deno scripts & report the job uptime? Conceptually it could make sense.
    unreachable!("osUptime permission check should always reject")
}

#[op]
fn op_os_uptime(state: &mut OpState) -> Result<u64, AnyError> {
    os_uptime(state)
}

#[op]
fn op_node_unstable_os_uptime(state: &mut OpState) -> Result<u64, AnyError> {
    os_uptime(state)
}
