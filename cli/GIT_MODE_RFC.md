# Cli Git mode

As part of the local dev improvements, improving the CLI behavior in the most common setup (used with Git Sync is primordial). Here are a collection of the proposed changes.

The current wmill.yaml contains sync settings such as:

```yaml
includes:
  - "**"
excludes:
  - "**"
skipResources: true
overrides:
  baseUrl:workspace_id:repo:
    skipVariables: true
```

## Branch mapping

When in a git repo, a branches section is MANDATORY. The CLI check for .git in every parent folder if branches section is missing and error if missing. Also error if current branch is not in the branches config.

```yaml
git_branches:
  branchName:
    baseUrl: X
    workspaceId: Y
```

We keep current saved user workspaces as "profiles" but their behavior changes and we refer to them as "user profile". They are stored in CONFIG_HOME/wmill

When a single user profile correspond to branch implied: baseUrl / workspaceId, it is chosen. When multiple, we store in the user config the last that was chosen for that compatible branch/baseUrl/workspaceId.

When no user profile is found for the given branch, we ask to create the profile.

## Workspace specific Overrides

```yaml
git_branches:
  branchName:
    ...
    overrides:
       skipResources: false
       ...
```

## Git sync overrides

```yaml
git_branches:
  branchName:
    ...
    overrides:
       skipResources: false
       ...
    promotionOverrides:
       skipResources: true
       ...
```

## Workspace specific items

```yaml
git_branches:
  branchName:
    ...
    specificItems:
      variables:
        - u/bar/foo
        - f/foo/**

```

When doing a pull/push, for all the items listed, instead of using the item at expected path, we have the item at `<path>.branchName.<extension>`

## Ephemeral workspaces

Commands:
`wmill ephemeral create foo`
`wmill ephemeral delete foo`

Create a git branch, switch to it, create an ephemeral workspace with the current user as admin, and add the profile in the user config

Branch names: `wm_ephemeral/<origin-branch>/<ephemeral-workspace-name>` (`/` in `original-branch` are replaced with `//`
Workspace name: `ephemeral-<branch_name>-<ephermal-workspace-name>`

Assume settings + baseUrl from the `<origin-branch>`

- a github action checks when those ephemeral branches are deleted and it delete the ephemeral workspace on Windmill.
- Ephemeral workspaces can be created from UI as an additional option from "Fork" button (and from workspace settings). You select one of the git sync connection (in non promotion mode) branch as origin.
- Ephemeral workspaces have a specific drawer accessible from home page that allows them to merge in both direction from their origin workspaces.
- Ephemeral workspaces have almost all their settings derived from original workspace, but git sync is setup in a specific way to point to the ephemeral branch
