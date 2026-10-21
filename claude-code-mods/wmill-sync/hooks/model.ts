import type { ItemKind, SyncDirection, SyncItem } from '../types'

export type ItemRef = { kind: ItemKind; path: string }

export const itemKey = (item: ItemRef) => `${item.kind}:${item.path}`

const FOLDER_ITEM = /^(.*?)(__flow|\.flow|__raw_app|\.raw_app|__app|\.app)\//
const TYPED_FILE = /^(.*)\.(variable|resource|resource-type|schedule|script)\.(yaml|json|lock)$/
const TRIGGER_FILE = /^(.*)\.[a-z0-9]+_trigger\.(yaml|json)$/
const RESOURCE_FILE = /^(.*)\.resource\.file\.[^/]+$/
// Script code files whose language is spelled with two extensions.
const COMPOUND_CODE = /\.(pg|my|bq|sf|ms|odb|duckdb|oracle|fetch|bun|deno|native|nativets)\.[a-z0-9]+$/
const ROOTS = ['f/', 'u/', 'g/']

const TYPED_KIND: Record<string, ItemKind> = {
  variable: 'variable',
  resource: 'resource',
  'resource-type': 'resource_type',
  schedule: 'schedule',
  script: 'script',
}

/**
 * The workspace item a project file belongs to, from wmill's on-disk layout;
 * undefined for files sync never takes (wmill.yaml, node_modules, docs).
 */
export const itemOfFile = (file: string): ItemRef | undefined => {
  const rel = file.replace(/^\.\//, '')
  if (!ROOTS.some(root => rel.startsWith(root))) return undefined
  if (rel.split('/').some(part => part === 'node_modules' || part.startsWith('.'))) {
    return undefined
  }

  const folderItem = FOLDER_ITEM.exec(rel)
  if (folderItem) {
    const suffix = folderItem[2] ?? ''
    const kind: ItemKind = suffix.includes('flow') ? 'flow' : 'app'
    return { kind, path: folderItem[1] ?? '' }
  }
  if (rel.endsWith('/folder.meta.yaml') || rel.endsWith('/folder.meta.json')) {
    return { kind: 'folder', path: rel.slice(0, rel.lastIndexOf('/')) }
  }
  const trigger = TRIGGER_FILE.exec(rel)
  if (trigger) return { kind: 'trigger', path: trigger[1] ?? '' }
  const resourceFile = RESOURCE_FILE.exec(rel)
  if (resourceFile) return { kind: 'resource', path: resourceFile[1] ?? '' }
  const typed = TYPED_FILE.exec(rel)
  if (typed) {
    const kind = TYPED_KIND[typed[2] ?? '']
    return kind ? { kind, path: typed[1] ?? '' } : undefined
  }
  if (/\.(yaml|yml|json|md|lock)$/.test(rel)) return undefined
  const base = rel.slice(rel.lastIndexOf('/') + 1)
  if (!base.includes('.')) return undefined
  const path = COMPOUND_CODE.test(rel) ? rel.replace(COMPOUND_CODE, '') : rel.slice(0, rel.lastIndexOf('.'))
  return { kind: 'script', path }
}

export type WorkspaceBinding = {
  name: string
  baseUrl?: string
  workspaceId?: string
  gitBranch?: string
}

/**
 * The `workspaces:` map of a wmill.yaml, read line by line: the mod has no
 * YAML library, and only the four scalar fields of each entry matter here.
 */
export const parseWorkspaceBindings = (yaml: string): WorkspaceBinding[] => {
  const lines = yaml.split('\n')
  const start = lines.findIndex(line => /^workspaces:/.test(line))
  if (start < 0) return []

  const bindings: WorkspaceBinding[] = []
  let current: WorkspaceBinding | undefined
  let entryIndent = -1
  for (const raw of lines.slice(start + 1)) {
    const line = raw.replace(/\s+#.*$/, '')
    if (line.trim() === '' || line.trim().startsWith('#')) continue
    const indent = line.length - line.trimStart().length
    if (indent === 0) break
    const match = /^(\s*)([A-Za-z0-9_.-]+):\s*(.*)$/.exec(line)
    if (!match) continue
    const value = (match[3] ?? '').replace(/^["']|["']$/g, '')
    if (entryIndent < 0) entryIndent = indent
    if (indent === entryIndent) {
      current = { name: match[2] ?? '' }
      bindings.push(current)
    } else if (current && indent > entryIndent && value !== '') {
      const field = match[2]
      if (field === 'baseUrl' || field === 'workspaceId' || field === 'gitBranch') {
        current[field] = value
      }
    }
  }
  return bindings.filter(binding => binding.name !== 'commonSpecificItems')
}

export const normalizeRemote = (url: string) => url.replace(/\/+$/, '') + '/'

export type BaseEntry = { localHash: string | null; version: string | null }

export type Observed = {
  /** Hash of the item's local files; null when it has none locally. */
  localHash: string | null
  /** The workspace's version token (script hash, flow edited_at, app version). */
  version: string | null
  /** Whether the item kind has a version token on the workspace at all. */
  hasVersion: boolean
  /** Whether git sees any of the item's local files as changed or untracked. */
  gitDirty: boolean | null
  existsLocally: boolean
  existsRemotely: boolean
}

/**
 * Which way an item that differs needs to go, from the last-synced record
 * when there is one and from git otherwise.
 */
export const classify = (
  base: BaseEntry | undefined,
  seen: Observed,
): { direction: SyncDirection; why: string } => {
  const local = seen.existsLocally ? 'edited locally' : 'deleted locally'
  const remote = seen.existsRemotely ? 'changed on the workspace' : 'deleted on the workspace'

  // A record is only written while both sides match, so the item existed on
  // both then: a change since is an edit or a deletion, never a creation.
  if (base) {
    const isLocalChanged = seen.localHash !== base.localHash
    const isRemoteChanged = seen.hasVersion ? seen.version !== base.version : !isLocalChanged
    if (isLocalChanged && isRemoteChanged) {
      return { direction: 'conflict', why: `${local}, and ${remote}` }
    }
    if (isLocalChanged) return { direction: 'push', why: local }
    if (isRemoteChanged) return { direction: 'pull', why: remote }
    return { direction: 'differs', why: 'differs, no change recorded on either side' }
  }

  if (seen.gitDirty === true) {
    return { direction: 'push', why: seen.existsRemotely ? local : 'new locally' }
  }
  if (seen.gitDirty === false) {
    return { direction: 'pull', why: seen.existsLocally ? remote : 'new on the workspace' }
  }
  return { direction: 'differs', why: 'differs, no sync recorded yet' }
}

export const KIND_LABEL: Record<ItemKind, string> = {
  script: 'script',
  flow: 'flow',
  app: 'app',
  variable: 'variable',
  resource: 'resource',
  resource_type: 'resource type',
  schedule: 'schedule',
  trigger: 'trigger',
  folder: 'folder',
}

export type DryRunChange = { type: string; path: string }

export type LocalItem = { ref: ItemRef; files: string[]; hash: string }

export const stripAnsi = (text: string) => text.replace(/\x1b\[[0-9;]*m/g, '')

export const lastLine = (text: string) =>
  text
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)
    .at(-1) ?? ''

/**
 * Groups the dry-run's file changes into items and decides each one's
 * direction; returns the new last-synced record alongside, with every item
 * that matches the workspace recorded at its current local hash and version.
 */
export const compare = (input: {
  changes: DryRunChange[]
  local: Map<string, LocalItem>
  versions: Map<string, string>
  base: Record<string, BaseEntry>
  gitDirty: Set<string> | null
}) => {
  const differing = new Map<string, { ref: ItemRef; files: Set<string>; types: Set<string> }>()
  for (const change of input.changes) {
    const ref = itemOfFile(change.path)
    if (!ref) continue
    const key = itemKey(ref)
    const entry = differing.get(key) ?? { ref, files: new Set<string>(), types: new Set<string>() }
    entry.files.add(change.path)
    entry.types.add(change.type)
    differing.set(key, entry)
  }

  const nextBase: Record<string, BaseEntry> = {}
  for (const [key, local] of input.local) {
    if (differing.has(key)) continue
    nextBase[key] = { localHash: local.hash, version: input.versions.get(key) ?? null }
  }

  const items: SyncItem[] = []
  for (const [key, entry] of differing) {
    const old = input.base[key]
    if (old) nextBase[key] = old
    const local = input.local.get(key)
    const files = [...new Set([...entry.files, ...(local?.files ?? [])])].sort()
    // Pull's view: "added" files exist only remotely, "deleted" only locally.
    const isOnlyRemote = [...entry.types].every(type => type === 'added')
    const isOnlyLocal = [...entry.types].every(type => type === 'deleted')
    const verdict = classify(old, {
      localHash: local?.hash ?? null,
      version: input.versions.get(key) ?? null,
      hasVersion: ['script', 'flow', 'app'].includes(entry.ref.kind),
      gitDirty: input.gitDirty === null ? null : files.some(file => input.gitDirty?.has(file)),
      existsLocally: local !== undefined && !isOnlyRemote,
      existsRemotely: !isOnlyLocal,
    })
    items.push({ key, kind: entry.ref.kind, path: entry.ref.path, files, ...verdict })
  }

  const order = { conflict: 0, push: 1, pull: 2, differs: 3 }
  items.sort((a, b) => order[a.direction] - order[b.direction] || a.path.localeCompare(b.path))
  return { items, nextBase }
}

export type FileDiff = { file: string; diff: string; isTruncated: boolean }

const SGR = /\x1b\[([0-9;]*)m/g
const HEADER = /^[~+-] [a-z_-]+ (\S+)$/
const CODE_LIMIT = 9_000

/**
 * The per-file diffs `wmill sync pull --dry-run --show-diffs` prints, as
 * unified hunks. The CLI marks lines by colour alone (red: the local text,
 * green: the workspace's, white: both), so the colours are the parse.
 */
export const parseShowDiffs = (raw: string): FileDiff[] => {
  const files: { file: string; lines: string[] }[] = []
  let color: 'local' | 'remote' | 'both' | null = null
  for (const line of raw.split('\n')) {
    let lineColor: typeof color = null
    let isVisibleSeen = false
    let last = 0
    for (const match of line.matchAll(SGR)) {
      if (!isVisibleSeen && match.index > last) isVisibleSeen = true
      last = match.index + match[0].length
      const codes = (match[1] ?? '').split(';')
      const next = codes.includes('31')
        ? 'local'
        : codes.includes('32')
          ? 'remote'
          : codes.includes('37')
            ? 'both'
            : codes.includes('0') || codes.includes('39')
              ? null
              : undefined
      if (next === undefined) continue
      if (!isVisibleSeen) lineColor = next
      color = next
    }
    const text = line.replace(SGR, '')
    const header = HEADER.exec(text)
    if (header) {
      files.push({ file: header[1] ?? '', lines: [] })
      continue
    }
    const current = files.at(-1)
    if (!current || text.startsWith('Dry run complete')) continue
    const kind = lineColor ?? color
    if (kind === null) {
      if (text.trim() === '') current.lines.push('')
      continue
    }
    current.lines.push(`${kind === 'local' ? '-' : kind === 'remote' ? '+' : ' '}${text}`)
  }

  return files.map(({ file, lines }) => {
    while (lines.length > 0 && lines.at(-1) === '') lines.pop()
    const body = lines.map(line => (line === '' ? ' ' : line))
    const removed = body.filter(line => !line.startsWith('+')).length
    const added = body.filter(line => !line.startsWith('-')).length
    const diff = [
      `--- local/${file}`,
      `+++ workspace/${file}`,
      `@@ -1,${removed} +1,${added} @@`,
      ...body,
    ].join('\n')
    return diff.length > CODE_LIMIT
      ? { file, diff: diff.slice(0, CODE_LIMIT), isTruncated: true }
      : { file, diff, isTruncated: false }
  })
}

/** The workspace items a tool call names: a file tool's path, or project paths in a shell command. */
export const touchedItems = (root: string, input: { file_path?: unknown; command?: unknown }) => {
  const paths: string[] = []
  if (typeof input.file_path === 'string' && input.file_path.startsWith(`${root}/`)) {
    paths.push(input.file_path.slice(root.length + 1))
  }
  if (typeof input.command === 'string') {
    for (const match of input.command.matchAll(/(?:^|[\s'"=(])((?:\.\/)?[fug]\/[^\s'";|&)]+)/g)) {
      paths.push((match[1] ?? '').replace(/^\.\//, ''))
    }
    for (const match of input.command.split(`${root}/`).slice(1)) {
      paths.push(match.split(/[\s'";|&)]/)[0] ?? '')
    }
  }
  const refs = new Map<string, ItemRef>()
  for (const path of paths) {
    const ref = itemOfFile(path)
    if (ref) refs.set(itemKey(ref), ref)
  }
  return [...refs.values()]
}

/**
 * The items a shell command syncs (a `wmill sync pull|push` that is not a
 * dry-run, or a per-item push): `all` when it takes the whole project.
 */
export const syncedByCommand = (command: string): ItemRef[] | 'all' | undefined => {
  const runs = command.split(/&&|\|\||;|\n/).map(part => part.trim())
  let synced: ItemRef[] | 'all' | undefined
  for (const run of runs) {
    if (/--dry-run/.test(run)) continue
    const sync = /\bwmill\s+sync\s+(pull|push)\b(.*)$/.exec(run)
    const single = /\bwmill\s+(script|flow|app)\s+push\s+(\S+)/.exec(run)
    if (sync) {
      const includes = /(?:-i|--includes)(?:\s+|=)("[^"]*"|'[^']*'|\S+)/.exec(sync[2] ?? '')
      if (!includes) return 'all'
      const paths = (includes[1] ?? '').replace(/^["']|["']$/g, '').split(',')
      const refs = paths
        .map(path => itemOfFile(path.trim()))
        .filter((ref): ref is ItemRef => ref !== undefined)
      synced = [...(synced === 'all' ? [] : (synced ?? [])), ...refs]
    } else if (single) {
      const path = (single[2] ?? '').replace(/^["']|["']$/g, '').replace(/^\.\//, '')
      // A flow or app is pushed by its folder, which names no file in it.
      const ref = itemOfFile(path) ?? itemOfFile(`${path.replace(/\/+$/, '')}/flow.yaml`)
      if (ref) synced = [...(synced === 'all' ? [] : (synced ?? [])), ref]
    }
  }
  return synced
}
