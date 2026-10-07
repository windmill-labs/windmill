export type ItemKind =
  | 'script'
  | 'flow'
  | 'app'
  | 'variable'
  | 'resource'
  | 'resource_type'
  | 'schedule'
  | 'trigger'
  | 'folder'

/**
 * push: changed locally since the last sync. pull: changed on the workspace.
 * conflict: both. differs: no last-sync record and no git history to tell.
 */
export type SyncDirection = 'push' | 'pull' | 'conflict' | 'differs'

export type SyncItem = {
  key: string
  kind: ItemKind
  path: string
  direction: SyncDirection
  /** One short phrase: "edited locally", "deleted on the workspace", ... */
  why: string
  /** Project-relative files the item spans, locally or on the workspace. */
  files: string[]
}

export type SyncTarget = {
  remote: string
  workspaceId: string
  /** Where the binding came from: a wmill.yaml entry or the active profile. */
  source: string
}

export type GitSyncInfo = {
  deployOnPush: boolean
  branch: string
  upstream: string | null
  ahead: number
  behind: number
}

export type SyncStatus = {
  phase: 'inactive' | 'checking' | 'ready' | 'error'
  root: string | null
  target: SyncTarget | null
  items: SyncItem[]
  checkedAt: number | null
  error: string | null
  gitSync: GitSyncInfo | null
}

export type DiffView = {
  item: SyncItem
  title: string
  files: { file: string; diff: string; isTruncated: boolean }[]
  error: string | null
}

declare module 'claude-code' {
  interface PluginState {
    'wmill-sync': {
      status: SyncStatus
      isExpanded: boolean
      busy: string | null
      diff: DiffView | null
    }
  }
}
