# Deploying the Ephemeral Backend Manager

This guide explains how to deploy the Windmill Ephemeral Backend Manager as a systemd service on a Linux machine.

## Prerequisites

Before running the installation script, ensure you have:

1. **Linux machine** with systemd (Ubuntu 20.04+, Debian 11+, etc.)
2. **User `sandbox`** created with appropriate permissions
3. **Repository cloned** to `/home/sandbox/ephemeral-backend/windmill`
4. **Required tools installed**:
   - Git
   - Docker (with `sandbox` user having access)
   - Bun (installed for the `sandbox` user)
   - Rust/Cargo (for building backends)
   - Bubblewrap (will be installed by the script if missing)
   - Cloudflared (will be installed by the script if missing)

## Architecture

The service runs as user `sandbox` and:

- Listens on port 8001 for HTTP requests
- Creates a Cloudflare tunnel for external access
- Updates GitHub Actions secrets with the tunnel URL
- Manages a pool of git worktrees for ephemeral backends
- Spawns ephemeral backends on-demand
- Automatically cleans up resources on shutdown

The worktree pool enables:

- Fast incremental compilation (reuses target directories)
- Efficient resource usage (no constant creation/deletion)
- Automatic discovery of existing worktrees on restart
