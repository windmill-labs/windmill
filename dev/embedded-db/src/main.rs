//! Lightweight dev-mode launcher (WIN-2130 experiment).
//!
//! Boots an embedded PostgreSQL (via `pglite-oxide` / oliphaunt — Postgres
//! compiled to WASIX, no Docker, no local Postgres install), exports
//! `DATABASE_URL`, and execs the command passed after `--` with that env set.
//! The embedded server lives in this parent process and is shut down when the
//! child exits.
//!
//! Usage:
//!   wm-embedded-db                       # just boot the DB and print the URL, then idle
//!   wm-embedded-db -- ./target/debug/windmill   # boot DB, then run windmill against it
//!
//! Env:
//!   WM_PGDATA   data dir (default: ./.wm-embedded-pgdata; empty => temporary)

use std::process::Command;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut child_cmd: Vec<String> = Vec::new();
    let mut seen_sep = false;
    for a in args.by_ref() {
        if !seen_sep && a == "--" {
            seen_sep = true;
            continue;
        }
        if seen_sep {
            child_cmd.push(a);
        }
    }

    let pgdata = std::env::var("WM_PGDATA").unwrap_or_else(|_| "./.wm-embedded-pgdata".to_string());

    eprintln!("[wm-embedded-db] booting embedded Postgres (pglite-oxide)...");
    let t0 = std::time::Instant::now();
    let mut builder = pglite_oxide::PgliteServer::builder();
    if pgdata.is_empty() {
        builder = builder.temporary();
    } else {
        builder = builder.path(&pgdata);
    }
    let server = builder.start()?;
    let url = server.database_url();
    eprintln!(
        "[wm-embedded-db] embedded Postgres ready in {:?}",
        t0.elapsed()
    );
    eprintln!("[wm-embedded-db] DATABASE_URL={url}");
    eprintln!(
        "[wm-embedded-db] NOTE: pglite-oxide serves ONE backend connection at a time; \
         point pools at a single connection."
    );

    if child_cmd.is_empty() {
        eprintln!("[wm-embedded-db] no command given; idling. Ctrl-C to stop.");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }

    eprintln!("[wm-embedded-db] exec: {}", child_cmd.join(" "));
    let status = Command::new(&child_cmd[0])
        .args(&child_cmd[1..])
        .env("DATABASE_URL", &url)
        .status()?;
    eprintln!("[wm-embedded-db] child exited: {status}");

    server.shutdown().ok();
    std::process::exit(status.code().unwrap_or(1));
}
