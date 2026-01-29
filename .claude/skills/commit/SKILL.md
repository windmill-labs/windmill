---
name: commit
user_invocable: true
description: Create a git commit with conventional commit format. MUST use anytime you want to commit changes.
---

# Git Commit Skill

Create a focused, single-line commit following conventional commit conventions.

## Instructions

1. **Analyze changes**: Run `git status` and `git diff` to understand what was modified
2. **Stage only modified files**: Add files individually by name. NEVER use `git add -A` or `git add .`
3. **Write commit message**: Follow the conventional commit format as a single line

## Conventional Commit Format

```
<type>: <description>
```

### Types
- `feat`: New feature or capability
- `fix`: Bug fix
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `docs`: Documentation only changes
- `style`: Formatting, missing semicolons, etc (no code change)
- `test`: Adding or correcting tests
- `chore`: Maintenance tasks, dependency updates, etc
- `perf`: Performance improvement

### Rules
- Message MUST be a single line (no multi-line messages)
- Description should be lowercase, imperative mood ("add" not "added")
- No period at the end
- Keep under 72 characters total

### Examples
```
feat: add token usage tracking for AI providers
fix: resolve null pointer in job executor
refactor: extract common validation logic
docs: update API endpoint documentation
chore: upgrade sqlx to 0.7
```

## Execution Steps

1. Run `git status` to see all changes
2. Run `git diff` to understand the changes in detail
3. Run `git log --oneline -5` to see recent commit style
4. Stage ONLY the modified/relevant files: `git add <file1> <file2> ...`
5. Create the commit with conventional format:
   ```bash
   git commit -m "<type>: <description>

   Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
   ```
6. Run `git status` to verify the commit succeeded
