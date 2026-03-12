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

If any `*_ee.rs` files were changed (check `git diff main...HEAD --name-only | grep '_ee\.rs$'`):

1. Find the EE worktree (same branch name as the current windmill branch):
   - `~/windmill-ee-private__worktrees/<branch>/` (workmux worktrees)
   - or `~/windmill-ee-private/` (main repo checkout)
2. In the EE repo: commit and push the changes on the same branch name
3. Create a companion PR:
   ```bash
   gh pr create --draft --repo windmill-labs/windmill-ee-private --title "<same title>" --body "$(cat <<'EOF'
   Companion PR for windmill-labs/windmill#<PR_NUMBER>

   ---
   Generated with [Claude Code](https://claude.com/claude-code)
   EOF
   )"
   ```
   Link to the windmill PR in the body.
4. Back in the windmill repo, update the EE ref:
   ```bash
   cd backend && bash write_latest_ee_ref.sh
   ```
5. Verify `ee-repo-ref.txt` contains the correct hash (from the feature branch, not main):
   ```bash
   cat backend/ee-repo-ref.txt
   ```
6. Commit `ee-repo-ref.txt`:
   ```bash
   git add backend/ee-repo-ref.txt && git commit -m "chore: update ee-repo-ref"
   ```
7. Push the updated windmill branch: `git push`
