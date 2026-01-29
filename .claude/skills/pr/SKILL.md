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
