# Autonomous Mode (Bypass Permissions)

When running in bypass/auto permission mode, follow these instructions to work end-to-end without human intervention.

## Available Tools

The Nix devShell provides these tools for documentation and testing:

- **`mmdc`** (mermaid-cli): Generate diagrams from Mermaid markup. Uses Nix-provided headless Chrome via `$PUPPETEER_EXECUTABLE_PATH`.
- **`asciinema`**: Record terminal sessions as `.cast` files for demo videos.
- **`playwright`** CLI: Take screenshots of the running frontend.

### When to Use Them

- **Designing a feature**: Use `mmdc` to generate Mermaid diagrams (architecture, data flow, sequence diagrams) during the planning phase. Include them in the PR description.
- **Frontend changes**: Take screenshots with the Playwright CLI after manual testing. Attach them to the PR.
- **CLI / terminal changes**: Record a demo with `asciinema` showing the feature in action. Attach to the PR.

### Quick Reference

```bash
# Generate a diagram
echo 'graph LR; A-->B; B-->C;' | mmdc -i - -o diagram.png

# Take a screenshot of a page
playwright screenshot --browser chromium http://localhost:3000 screenshot.png

# Record a terminal demo
asciinema rec demo.cast
# ... do the demo ...
# ctrl-d to stop
```

## Always Plan First

Even in bypass mode, **enter plan mode before starting non-trivial work**. Ask all important questions upfront:
- Clarify ambiguous requirements before writing code
- Identify which files, crates, and features are affected
- Read `docs/validation.md` to know what checks you'll need to run
- Break large features into stages — commit each stage separately

## Dev Environment (tmux)

Backend and frontend are running in tmux panes in the current session:

- **Pane 0**: This pane (Claude agent)
- **Pane 1**: Backend (`cargo watch -x run`)
- **Pane 2**: Frontend (`npm run dev`)

### Checking Logs

```bash
# Backend logs (last 50 lines)
tmux capture-pane -t .1 -p -S -50

# Frontend logs (last 50 lines)
tmux capture-pane -t .2 -p -S -50
```

Backend runs with `cargo watch`, so Rust changes auto-recompile — just check pane 1 logs for errors. No need to run `cargo check` separately during iteration if the backend pane shows a successful build.

## Manual Testing

After code changes compile and type-check, verify the feature works:

1. **Check backend logs** (`tmux capture-pane -t .1 -p -S -50`) — confirm no panics or errors
2. **Check frontend logs** (`tmux capture-pane -t .2 -p -S -50`) — confirm no build errors
3. **Use Playwright MCP** to test the UI flow:
   - Navigate to `http://localhost:3000/user/login`
   - Click "Log in without third-party"
   - Login with `admin@windmill.dev` / `changeme`
   - Navigate to the page affected by your change
   - Verify the feature works as expected
4. **Test edge cases**: empty states, error states, permissions

### Playwright Gotchas

- Backend takes ~60s to compile on first change; check logs for `health check completed`
- Frontend rebuilds in ~5s
- `critical_alerts` 404s are expected on CE builds (EE-only endpoint) — ignore them
- VSCode worker 404s are dev-mode artifacts — ignore them
- The `<Toggle>` component hides the checkbox (`sr-only`). Click the `<label>` wrapper, not the checkbox

## End-of-Task Summary

When done, provide:
- What was changed and why (files modified, approach taken)
- What checks passed (cargo check, npm run check, etc.)
- What was manually tested and the results
- **Screenshots** of UI changes (via `playwright screenshot`)
- **Terminal recordings** of CLI changes (via asciinema)
- Any known limitations or follow-up work needed

Upload images via pastebin (e.g., `curl -F 'file=@screenshot.png' https://0x0.st`) and include the URLs in the PR description or comments.
