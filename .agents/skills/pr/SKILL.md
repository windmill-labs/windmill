---
name: pr
user_invocable: true
description: Open a draft pull request on GitHub. MUST use when you want to create/open a PR.
---

# Pull Request Skill

Create a draft pull request with a clear title and explicit description of changes.

## Instructions

1. **Analyze branch changes**: Understand all commits since diverging from main
2. **Push to remote**: Ensure all commits are pushed
3. **Create draft PR**: Always open as draft for review before merging

## PR Title Format

Follow conventional commit format for the PR title:
```
<type>: <description>
```

### Types
- `feat`: New feature or capability
- `fix`: Bug fix
- `refactor`: Code restructuring
- `docs`: Documentation changes
- `chore`: Maintenance tasks
- `perf`: Performance improvements

### Title Rules
- Keep under 70 characters
- Use lowercase, imperative mood
- No period at the end
- If `*_ee.rs` files were modified, prefix with `[ee]`: `[ee] <type>: <description>`

## PR Body Format

The body MUST be explicit about what changed. Structure:

```markdown
## Summary
<Clear description of what this PR does and why>

## Changes
- <Specific change 1>
- <Specific change 2>
- <Specific change 3>

## Test plan
- [ ] <How to verify change 1>
- [ ] <How to verify change 2>

---
Generated with [Claude Code](https://claude.com/claude-code)
```

## Execution Steps

1. Run `git status` to check for uncommitted changes
2. Run `git log main..HEAD --oneline` to see all commits in this branch
3. Run `git diff main...HEAD` to see the full diff against main
4. Check if remote branch exists and is up to date:
   ```bash
   git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null || echo "no upstream"
   ```
5. Push to remote if needed: `git push -u origin HEAD`
6. Create draft PR using gh CLI:
   ```bash
   gh pr create --draft --title "<type>: <description>" --body "$(cat <<'EOF'
   ## Summary
   <description>

   ## Changes
   - <change 1>
   - <change 2>

   ## Test plan
   - [ ] <test 1>
   - [ ] <test 2>

   ---
   Generated with [Claude Code](https://claude.com/claude-code)
   EOF
   )"
   ```
7. Return the PR URL to the user

## EE Companion PR (when `*_ee.rs` files were modified)

The `*_ee.rs` files in the windmill repo are **symlinks** to `windmill-ee-private` — changes won't appear in `git diff` of the windmill repo. Instead, check the EE repo for uncommitted or unpushed changes.

Follow the full EE PR workflow in `docs/enterprise.md`. The key PR-specific details:

1. Find the EE repo/worktree: see "Finding the EE Repo" in `docs/enterprise.md`
2. Check for changes: `git -C <ee-path> status --short`
   - If there are no changes in the EE repo, skip this entire section
3. Follow steps 1–5 from the "EE PR Workflow" in `docs/enterprise.md`
4. Create the companion PR (title does NOT get the `[ee]` prefix):
   ```bash
   gh pr create --draft --repo windmill-labs/windmill-ee-private --title "<type>: <description>" --body "$(cat <<'EOF'
   Companion PR for windmill-labs/windmill#<PR_NUMBER>

   ---
   Generated with [Claude Code](https://claude.com/claude-code)
   EOF
   )"
   ```
5. Commit `ee-repo-ref.txt` and push the updated windmill branch
