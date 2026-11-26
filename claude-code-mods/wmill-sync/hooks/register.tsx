import { atom, read, update } from 'claude-code'
import type { EngineInterface, Register } from 'claude-code'

import type { GitSyncInfo, SyncItem, SyncStatus, SyncTarget } from '../types'
import {
  type BaseEntry,
  compare,
  type DryRunChange,
  itemKey,
  itemOfFile,
  type ItemRef,
  KIND_LABEL,
  lastLine,
  type LocalItem,
  normalizeRemote,
  parseWorkspaceBindings,
  syncedByCommand,
  touchedItems,
  parseShowDiffs,
  stripAnsi,
} from './model'

type $ = EngineInterface

export type Target = SyncTarget & { token: string }

export class SyncError extends Error {}

const WMILL_DIRS = ['/opt/homebrew/bin', '/usr/local/bin', '~/.bun/bin', '~/.npm-global/bin', '~/.local/bin']

/** The folder holding wmill.yaml, walking up from the session's root. */
export const findProjectRoot = async ($: $): Promise<string | null> => {
  let dir = (await $.session.root()).replace(/\/+$/, '')
  while (dir !== '') {
    if (await $.fs.exists(`${dir}/wmill.yaml`)) return dir
    dir = dir.slice(0, dir.lastIndexOf('/'))
  }
  return null
}

const home = async ($: $) => (await $.env.get('HOME')) ?? ''

/**
 * Desktop and IDE hosts start Claude Code without the login shell's PATH, so
 * `wmill` (and the `node` its shebang asks for) are looked up in the usual
 * install folders too.
 */
const childEnv = async ($: $, isColored: boolean) => {
  const homeDir = await home($)
  const extra = WMILL_DIRS.map(dir => dir.replace(/^~/, homeDir))
  const path = (await $.env.get('PATH')) ?? '/usr/bin:/bin'
  return { PATH: [...extra, ...path.split(':')].join(':'), FORCE_COLOR: isColored ? '1' : '0' }
}

export const runWmill = async (
  $: $,
  wmill: string,
  root: string,
  args: string[],
  timeoutMs = 120_000,
  isColored = false,
) => {
  const result = await $.process
    .run([wmill, ...args], { cwd: root, env: await childEnv($, isColored), timeoutMs })
    .catch((error: unknown) => {
      throw new SyncError(
        `could not run ${wmill} (${String(error).slice(0, 120)}). Install the wmill CLI or set wmill-sync's "wmill executable" option.`,
      )
    })
  if (isColored) return result
  return { ...result, stdout: stripAnsi(result.stdout), stderr: stripAnsi(result.stderr) }
}

const runGit = async ($: $, root: string, args: string[]) => {
  const result = await $.process.run(['git', '-C', root, ...args]).catch(() => undefined)
  return result?.exitCode === 0 ? result.stdout : undefined
}

// ---------------------------------------------------------------- target

const configDir = async ($: $) => {
  const override = await $.env.get('WMILL_CONFIG_DIR')
  if (override) return `${override.replace(/\/+$/, '')}/windmill`
  const homeDir = await home($)
  if (await $.fs.exists(`${homeDir}/Library/Preferences`)) {
    return `${homeDir}/Library/Preferences/windmill`
  }
  const xdg = await $.env.get('XDG_CONFIG_HOME')
  return `${xdg ?? `${homeDir}/.config`}/windmill`
}

type Profile = { name: string; remote: string; workspaceId: string; token: string }

const readProfiles = async ($: $, dir: string): Promise<Profile[]> => {
  const text = await $.fs.read(`${dir}/remotes.ndjson`).catch(() => '')
  return text
    .split('\n')
    .filter(line => line.trim() !== '')
    .flatMap(line => {
      try {
        return [JSON.parse(line) as Profile]
      } catch {
        return []
      }
    })
}

/**
 * The workspace this project syncs with, resolved the way the CLI does for
 * the common setups: a wmill.yaml binding for the current git branch, else
 * the active profile. The token is the CLI's own and never leaves memory.
 */
export const resolveTarget = async ($: $, root: string): Promise<Target> => {
  const dir = await configDir($)
  const profiles = await readProfiles($, dir)
  const yaml = await $.fs.read(`${root}/wmill.yaml`).catch(() => '')
  const bindings = parseWorkspaceBindings(yaml)

  if (bindings.length > 0) {
    const branch = (await runGit($, root, ['rev-parse', '--abbrev-ref', 'HEAD']))?.trim()
    const binding =
      bindings.find(one => (one.gitBranch ?? one.name) === branch) ??
      (bindings.length === 1 ? bindings[0] : undefined)
    if (binding) {
      const workspaceId = binding.workspaceId ?? binding.name
      const profile =
        profiles.find(
          one =>
            one.workspaceId === workspaceId &&
            (!binding.baseUrl || normalizeRemote(one.remote) === normalizeRemote(binding.baseUrl)),
        ) ?? profiles.find(one => one.name === binding.name)
      if (!profile) {
        throw new SyncError(
          `no wmill profile for workspace "${workspaceId}". Run \`wmill workspace add\` for it.`,
        )
      }
      return {
        remote: normalizeRemote(binding.baseUrl ?? profile.remote),
        workspaceId,
        token: profile.token,
        source: `wmill.yaml "${binding.name}"`,
      }
    }
  }

  const active = (await $.fs.read(`${dir}/activeWorkspace`).catch(() => '')).trim()
  const profile = profiles.find(one => one.name === active)
  if (!profile) {
    throw new SyncError('no active wmill workspace. Run `wmill workspace add` or `wmill workspace switch`.')
  }
  return {
    remote: normalizeRemote(profile.remote),
    workspaceId: profile.workspaceId,
    token: profile.token,
    source: `profile "${profile.name}"`,
  }
}

// ---------------------------------------------------------------- remote

const api = async ($: $, target: Target, path: string) => {
  const url = `${target.remote}api/w/${target.workspaceId}/${path}`
  const response = await $.http
    .fetch(url, { headers: { authorization: `Bearer ${target.token}` } })
    .catch(() => {
      throw new SyncError(`cannot reach ${target.remote}; is the instance up?`)
    })
  if (response.status === 401 || response.status === 403) {
    throw new SyncError(
      `the wmill token for ${target.workspaceId} was refused. Run \`wmill workspace add\` again.`,
    )
  }
  if (!response.ok) {
    throw new SyncError(`${path} answered ${response.status}`)
  }
  return JSON.parse(response.text) as unknown
}

const listAll = async ($: $, target: Target, endpoint: string) => {
  const rows: Record<string, unknown>[] = []
  for (let page = 1; page < 50; page += 1) {
    const batch = (await api($, target, `${endpoint}?per_page=1000&page=${page}`)) as Record<
      string,
      unknown
    >[]
    rows.push(...batch)
    if (batch.length < 1000) break
  }
  return rows
}

/** Version tokens of the kinds that have one, keyed by item key. */
export const remoteVersions = async ($: $, target: Target) => {
  const [scripts, flows, apps] = await Promise.all([
    listAll($, target, 'scripts/list'),
    listAll($, target, 'flows/list'),
    listAll($, target, 'apps/list'),
  ])
  const versions = new Map<string, string>()
  for (const row of scripts)
    versions.set(itemKey({ kind: 'script', path: String(row.path) }), String(row.hash))
  for (const row of flows)
    versions.set(itemKey({ kind: 'flow', path: String(row.path) }), String(row.edited_at))
  for (const row of apps) versions.set(itemKey({ kind: 'app', path: String(row.path) }), String(row.version))
  return versions
}

export const VERSIONED = new Set(['script', 'flow', 'app'])

export const gitSyncMode = async ($: $, target: Target) => {
  const mode = (await api($, target, 'workspaces/git_sync_deploy_mode').catch(() => undefined)) as
    { configured?: boolean; deploy_on_push?: boolean } | undefined
  return mode?.configured ? { deployOnPush: mode.deploy_on_push === true } : null
}

export const gitAheadBehind = async (
  $: $,
  root: string,
  deployOnPush: boolean,
  isFetching: boolean,
): Promise<GitSyncInfo | null> => {
  const branch = (await runGit($, root, ['rev-parse', '--abbrev-ref', 'HEAD']))?.trim()
  if (!branch) return null
  if (isFetching) await runGit($, root, ['fetch', '--quiet'])
  const upstream = (await runGit($, root, ['rev-parse', '--abbrev-ref', '@{upstream}']))?.trim() ?? null
  if (!upstream) return { deployOnPush, branch, upstream: null, ahead: 0, behind: 0 }
  const counts = (
    await runGit($, root, ['rev-list', '--left-right', '--count', `HEAD...${upstream}`])
  )?.trim()
  const [ahead = 0, behind = 0] = (counts ?? '0 0').split(/\s+/).map(Number)
  return { deployOnPush, branch, upstream, ahead, behind }
}

// ---------------------------------------------------------------- local

type FileStamp = { mtimeMs: number; size: number; hash: string }

const fileHashes = new Map<string, FileStamp>()

const sha256 = async (text: string) => {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text))
  return [...new Uint8Array(digest)].map(byte => byte.toString(16).padStart(2, '0')).join('')
}

const walk = async (
  $: $,
  root: string,
  rel: string,
  out: { rel: string; mtimeMs: number; size: number }[],
) => {
  const entries = await $.fs.list(`${root}/${rel}`).catch(() => [])
  for (const entry of entries) {
    if (entry.name.startsWith('.') || entry.name === 'node_modules') continue
    const child = `${rel}/${entry.name}`
    if (entry.kind === 'dir') await walk($, root, child, out)
    else if (entry.kind === 'file') out.push({ rel: child, mtimeMs: entry.mtimeMs, size: entry.size })
  }
}

/**
 * Every item with files in the project, hashed over its files' contents.
 * A file's hash is reused while its mtime and size stay the same.
 */
export const scanLocal = async ($: $, root: string) => {
  const files: { rel: string; mtimeMs: number; size: number }[] = []
  for (const top of ['f', 'u', 'g']) {
    if (await $.fs.exists(`${root}/${top}`)) await walk($, root, top, files)
  }

  const byItem = new Map<string, { ref: ItemRef; files: string[] }>()
  for (const file of files) {
    const ref = itemOfFile(file.rel)
    if (!ref) continue
    const key = itemKey(ref)
    const entry = byItem.get(key) ?? { ref, files: [] }
    entry.files.push(file.rel)
    byItem.set(key, entry)
  }

  const stamps = new Map(files.map(file => [file.rel, file]))
  const items = new Map<string, LocalItem>()
  for (const [key, entry] of byItem) {
    const parts: string[] = []
    for (const rel of entry.files.sort()) {
      const stamp = stamps.get(rel)
      const absolute = `${root}/${rel}`
      const cached = fileHashes.get(absolute)
      let hash = cached?.hash
      if (!cached || !stamp || cached.mtimeMs !== stamp.mtimeMs || cached.size !== stamp.size) {
        hash = await sha256(await $.fs.read(absolute).catch(() => ''))
        fileHashes.set(absolute, { mtimeMs: stamp?.mtimeMs ?? 0, size: stamp?.size ?? 0, hash })
      }
      parts.push(`${rel}:${hash}`)
    }
    items.set(key, { ref: entry.ref, files: entry.files, hash: await sha256(parts.join('\n')) })
  }
  const fingerprint = files
    .map(file => `${file.rel}:${file.mtimeMs}:${file.size}`)
    .sort()
    .join('\n')
  return { items, fingerprint }
}

/** Project-relative paths git reports as modified, deleted or untracked; null outside git. */
export const gitDirtyFiles = async ($: $, root: string) => {
  const prefix = await runGit($, root, ['rev-parse', '--show-prefix'])
  if (prefix === undefined) return null
  const status = await runGit($, root, ['status', '--porcelain=v1', '-z', '--untracked-files=all', '--', '.'])
  if (status === undefined) return null
  const dirty = new Set<string>()
  const records = status.split('\0')
  for (let index = 0; index < records.length; index += 1) {
    const record = records[index] ?? ''
    if (record.length < 4) continue
    const code = record.slice(0, 2)
    const path = record.slice(3)
    dirty.add(path.startsWith(prefix.trim()) ? path.slice(prefix.trim().length) : path)
    // A rename's record is followed by its source path.
    if (code.startsWith('R') || code.startsWith('C')) index += 1
  }
  return dirty
}

// ---------------------------------------------------------------- compare

export const dryRunPull = async ($: $, wmill: string, root: string): Promise<DryRunChange[]> => {
  const run = await runWmill($, wmill, root, ['sync', 'pull', '--dry-run', '--json-output'])
  const start = run.stdout.search(/^\{/m)
  if (run.exitCode !== 0 || start < 0) {
    throw new SyncError(`wmill sync pull --dry-run failed: ${lastLine(run.stderr) || lastLine(run.stdout)}`)
  }
  const parsed = JSON.parse(run.stdout.slice(start)) as { changes?: DryRunChange[] }
  return parsed.changes ?? []
}

const POLL_MS = 120_000
const EDIT_SETTLE_MS = 2_000
// A file tool touching an item without a version rechecks first past this age.
const STALE_MS = 60_000
// Stands for "whatever the files held right after a sync command": never a real hash.
const SYNCED_BY_COMMAND = 'synced-by-command'
// Variables, resources and schedules carry no version, so only a dry-run
// sees them change remotely: every this-many polls one runs regardless.
const FULL_CHECK_EVERY = 5

const EMPTY: SyncStatus = {
  phase: 'inactive',
  root: null,
  target: null,
  items: [],
  checkedAt: null,
  error: null,
  gitSync: null,
}

const status = atom({ plugin: 'wmill-sync', key: 'status' } as const, EMPTY)
const isExpanded = atom({ plugin: 'wmill-sync', key: 'isExpanded' } as const, false)
const busy = atom({ plugin: 'wmill-sync', key: 'busy' } as const, null)
const diffView = atom({ plugin: 'wmill-sync', key: 'diff' } as const, null)
const DIFF_PANE = 'wmill-sync-diff'

const MARK = { push: '↑', pull: '↓', conflict: '!', differs: '≠' } as const
const COLOR = { push: 'cyan', pull: 'yellow', conflict: 'red', differs: 'magenta' } as const

const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? '' : 's'}`

export const summarize = (items: SyncItem[]) => {
  const count = (direction: SyncItem['direction']) =>
    items.filter(item => item.direction === direction).length
  return [
    count('push') > 0 && `${count('push')} to push`,
    count('pull') > 0 && `${count('pull')} to pull`,
    count('conflict') > 0 && plural(count('conflict'), 'conflict'),
    count('differs') > 0 && `${count('differs')} differing`,
  ]
    .filter(Boolean)
    .join(' · ')
}

const fit = (text: string, width: number, cut: 'middle' | 'end') => {
  if (text.length <= width) return text.padEnd(width)
  if (width <= 1) return '…'.slice(0, width)
  if (cut === 'end') return `${text.slice(0, width - 1)}…`
  const head = Math.ceil((width - 1) / 2)
  return `${text.slice(0, head)}…${text.slice(text.length - (width - 1 - head))}`
}

const describe = (item: SyncItem) => `${KIND_LABEL[item.kind]} ${item.path}`

const baseKey = (root: string, remote: string, workspaceId: string) => `base:${root}|${remote}|${workspaceId}`

let wmill = 'wmill'
let isRefreshing = false
let isRefreshQueued = false
let pollCount = 0
let lastSeen: { versions: Map<string, string>; fingerprint: string } | null = null
let gitSync: { remote: string; mode: { deployOnPush: boolean } | null } | null = null
let editTimer: { cancel: () => void } | null = null
const noted = new Set<string>()

async function setStatus($: $, patch: Partial<SyncStatus>) {
  await update($, status, current => ({ ...current, ...patch }))
}

/**
 * The full comparison: scan local files, read workspace versions, run the
 * CLI's pull dry-run, classify each differing item and move the last-synced
 * record forward for every item that matches.
 */
async function refresh($: $, options: { isFetchingGit?: boolean } = {}) {
  if (isRefreshing) {
    isRefreshQueued = true
    return
  }
  isRefreshing = true
  try {
    const root = await findProjectRoot($)
    if (!root) {
      await setStatus($, { ...EMPTY })
      return
    }
    await setStatus($, { phase: 'checking', root })
    const target = await resolveTarget($, root)
    const [{ items: local, fingerprint }, versions, changes, dirty] = await Promise.all([
      scanLocal($, root),
      remoteVersions($, target),
      dryRunPull($, wmill, root),
      gitDirtyFiles($, root),
    ])
    if (gitSync?.remote !== `${target.remote}|${target.workspaceId}`) {
      gitSync = { remote: `${target.remote}|${target.workspaceId}`, mode: await gitSyncMode($, target) }
    }
    const key = baseKey(root, target.remote, target.workspaceId)
    const base = ((await $.store.get(key)) ?? {}) as Record<string, BaseEntry>
    const { items, nextBase } = compare({ changes, local, versions, base, gitDirty: dirty })
    await $.store.set(key, nextBase)
    const git = gitSync.mode
      ? await gitAheadBehind($, root, gitSync.mode.deployOnPush, options.isFetchingGit === true)
      : null
    lastSeen = { versions, fingerprint }
    await setStatus($, {
      phase: 'ready',
      root,
      target: { remote: target.remote, workspaceId: target.workspaceId, source: target.source },
      items,
      checkedAt: await $.clock.now(),
      error: null,
      gitSync: git,
    })
  } catch (error) {
    const message =
      error instanceof SyncError ? error.message : `check failed: ${String(error).slice(0, 200)}`
    await setStatus($, { phase: 'error', error: message, checkedAt: await $.clock.now() })
  } finally {
    isRefreshing = false
    if (isRefreshQueued) {
      isRefreshQueued = false
      void refresh($)
    }
  }
}

/** The cheap timer check: a dry-run only when something moved on either side. */
async function poll($: $) {
  pollCount += 1
  const current = await read($, status)
  if (current.phase === 'inactive' || !current.root) return
  if (pollCount % FULL_CHECK_EVERY === 0 || !lastSeen) {
    await refresh($, { isFetchingGit: true })
    return
  }
  try {
    const target = await resolveTarget($, current.root)
    const [versions, { fingerprint }] = await Promise.all([
      remoteVersions($, target),
      scanLocal($, current.root),
    ])
    const hasMoved =
      JSON.stringify([...versions]) !== JSON.stringify([...lastSeen.versions]) ||
      fingerprint !== lastSeen.fingerprint
    if (hasMoved || current.phase === 'error') await refresh($, { isFetchingGit: true })
  } catch {
    await refresh($)
  }
}

/**
 * Rechecks once edits settle. A shell command may have written project files
 * through any program, so after one the local files are rescanned (no network)
 * and the full check runs only when they moved.
 */
function refreshSoon($: $, options: { isLocalOnly?: boolean } = {}) {
  editTimer?.cancel()
  editTimer = $.clock.after(EDIT_SETTLE_MS, () => void refreshIfMoved($, options.isLocalOnly === true))
}

async function refreshIfMoved($: $, isLocalOnly: boolean) {
  const { root } = await read($, status)
  if (isLocalOnly && root && lastSeen) {
    const { fingerprint } = await scanLocal($, root)
    if (fingerprint === lastSeen.fingerprint) return
  }
  await refresh($)
}

/**
 * Records items a shell command just pulled or pushed as matching the
 * workspace at its current version. A command that goes on to edit them
 * (`pull && sed …`) leaves them differing, and that reads as a local edit
 * rather than both sides changing.
 */
async function markSynced($: $, synced: ItemRef[] | 'all') {
  const current = await read($, status)
  if (!current.root) return
  try {
    const target = await resolveTarget($, current.root)
    const versions = await remoteVersions($, target)
    const key = baseKey(current.root, target.remote, target.workspaceId)
    const base = ((await $.store.get(key)) ?? {}) as Record<string, BaseEntry>
    const keys = synced === 'all' ? current.items.map(item => item.key) : synced.map(itemKey)
    for (const itemKey of keys)
      base[itemKey] = { localHash: SYNCED_BY_COMMAND, version: versions.get(itemKey) ?? null }
    await $.store.set(key, base)
  } catch {
    // The next full check falls back to comparing against the older record.
  }
}

/** Whether the workspace version of any of these items moved since the last full check: three list calls. */
async function hasRemoteMoved($: $, keys: string[]) {
  const { root } = await read($, status)
  if (!root || !lastSeen) return true
  try {
    const versions = await remoteVersions($, await resolveTarget($, root))
    return keys.some(key => versions.get(key) !== lastSeen?.versions.get(key))
  } catch {
    return false
  }
}

async function act($: $, label: string, argv: string[], timeoutMs = 600_000) {
  if ((await read($, busy)) !== null) return
  const { root } = await read($, status)
  if (!root) return
  await update($, busy, () => label)
  try {
    const run = await runWmill(
      $,
      argv[0] === 'git' ? 'git' : wmill,
      root,
      argv[0] === 'git' ? argv.slice(1) : argv,
      timeoutMs,
    )
    if (run.exitCode === 0) {
      $.ui.toast(`wmill-sync: ${label} done`)
    } else {
      $.ui.toast(`wmill-sync: ${label} failed: ${lastLine(run.stderr) || lastLine(run.stdout)}`, {
        timeoutMs: 10_000,
      })
    }
  } catch (error) {
    $.ui.toast(`wmill-sync: ${label} failed: ${error instanceof Error ? error.message : String(error)}`)
  } finally {
    await update($, busy, () => null)
  }
  await refresh($)
}

function push($: $, items: SyncItem[]) {
  return act($, items.length === 1 ? `push ${items[0]?.path}` : `push ${plural(items.length, 'item')}`, [
    'sync',
    'push',
    '--yes',
    '--message',
    'Pushed from Claude Code (wmill-sync)',
    '-i',
    items.flatMap(item => item.files).join(','),
  ])
}

function pull($: $, items: SyncItem[]) {
  return act($, items.length === 1 ? `pull ${items[0]?.path}` : `pull ${plural(items.length, 'item')}`, [
    'sync',
    'pull',
    '--yes',
    '-i',
    items.flatMap(item => item.files).join(','),
  ])
}

async function showDiff($: $, item: SyncItem) {
  const { root } = await read($, status)
  if (!root) return
  const title = `${describe(item)}: local (−) vs workspace (+)`
  await update($, diffView, () => ({ item, title, files: [], error: 'Loading the workspace version…' }))
  await $.ui.open({ id: DIFF_PANE, title: `diff ${item.path}`, focus: true, closeOnEscape: true })
  try {
    const run = await runWmill(
      $,
      wmill,
      root,
      ['sync', 'pull', '--dry-run', '--show-diffs', '-i', item.files.join(',')],
      120_000,
      true,
    )
    const files = parseShowDiffs(run.stdout)
    const error =
      run.exitCode !== 0
        ? lastLine(stripAnsi(run.stderr)) || lastLine(stripAnsi(run.stdout))
        : files.length === 0
          ? 'No differences left: the item matches the workspace now.'
          : null
    await update($, diffView, () => ({ item, title, files, error }))
  } catch (caught) {
    await update($, diffView, () => ({
      item,
      title,
      files: [],
      error: caught instanceof Error ? caught.message : String(caught),
    }))
  }
}

async function closeDiff($: $) {
  await $.ui.close({ id: DIFF_PANE })
}

async function askToMerge($: $, item: SyncItem) {
  const { target } = await read($, status)
  const files = item.files.join(', ')
  await $.prompt.submit({
    text: [
      `The Windmill ${describe(item)} changed both in this project and on the workspace ${target?.workspaceId ?? ''}.`,
      `Its files: ${files}.`,
      `Show me what changed remotely without overwriting my files (\`wmill sync pull --dry-run --show-diffs -i ${item.files.join(',')}\`),`,
      'then merge the workspace changes into the local files, keeping my local edits.',
      'Do not push: once I have reviewed the merge I will press "Keep local" in the wmill-sync panel, which pushes it.',
    ].join(' '),
  })
}

export const register: Register = (on, options) => {
  wmill = typeof options.wmillPath === 'string' && options.wmillPath !== '' ? options.wmillPath : 'wmill'

  on('session.start', async ($, e, next) => {
    const started = await next(e)
    $.ui.status(undefined)
    await $.command.register({
      name: 'wmill-sync',
      description: 'Check what differs between this wmill project and its Windmill workspace',
    })
    void refresh($)
    $.clock.every(POLL_MS, () => void poll($))
    return started
  })

  on('command.run', { command: 'wmill-sync' }, async $ => {
    await refresh($)
    const current = await read($, status)
    if (current.phase === 'inactive') return { text: 'No wmill.yaml in this project or above it.' }
    if (current.phase === 'error') return { text: current.error ?? 'The check failed.' }
    await update($, isExpanded, () => true)
    const where = `workspace ${current.target?.workspaceId} on ${current.target?.remote}`
    if (current.items.length === 0) return { text: `In sync with ${where}.` }
    const lines = current.items.map(item => `${MARK[item.direction]} ${describe(item)}: ${item.why}`)
    return { text: [`${summarize(current.items)} against ${where}`, ...lines].join('\n') }
  })

  // Tell the model when a file it works on belongs to an item that is out of
  // sync, and recheck once its edits settle.
  on('tool.call', async ($, e, next) => {
    const tool = String(e.tool)
    const input = e as unknown as { file_path?: unknown; command?: unknown }
    const before = await read($, status)
    const refs = before.root ? touchedItems(before.root, input) : []
    if (refs.length > 0 && before.phase === 'ready') {
      // Recheck before the model works on an item someone may have changed on
      // the workspace: by its version where it has one, by age otherwise.
      const isStale = (await $.clock.now()) - (before.checkedAt ?? 0) > STALE_MS
      const versioned = refs.filter(ref => ref.kind === 'script' || ref.kind === 'flow' || ref.kind === 'app')
      const hasMoved =
        (versioned.length < refs.length && isStale) ||
        (versioned.length > 0 && (await hasRemoteMoved($, versioned.map(itemKey))))
      if (hasMoved) await refresh($)
    }

    const result = await next(e)
    if (tool === 'Bash') {
      // A sync command moves the last-synced record: check at once, before a
      // following edit makes the item differ again.
      const isSyncCommand = typeof input.command === 'string' && /\b(wmill|git)\b/.test(input.command)
      const synced = typeof input.command === 'string' ? syncedByCommand(input.command) : undefined
      if (synced && !result.isError && result.deny === undefined) await markSynced($, synced)
      if (isSyncCommand) await refresh($)
      else refreshSoon($, { isLocalOnly: true })
    } else if (refs.length > 0 && ['Edit', 'Write', 'MultiEdit', 'NotebookEdit'].includes(tool)) {
      refreshSoon($)
    }
    if (refs.length === 0 || result.deny !== undefined) return result

    const current = await read($, status)
    const notes = refs.flatMap(ref => {
      const item = current.items.find(one => one.key === itemKey(ref))
      if (!item || item.direction === 'push') return []
      const noteKey = `${item.key}|${item.direction}|${item.why}`
      if (noted.has(noteKey)) return []
      noted.add(noteKey)
      const advice = {
        pull: `Pull it before editing (\`wmill sync pull --yes -i ${item.files.join(',')}\`), or a later push overwrites the workspace version.`,
        conflict: 'Both sides changed: do not push it. Show the user the remote diff and merge first.',
        differs:
          'Which side changed is unknown: check `wmill sync pull --dry-run --show-diffs` before pushing or pulling.',
      }[item.direction]
      return [
        `wmill-sync: ${describe(item)} differs from workspace ${current.target?.workspaceId} (${item.why}). ${advice}`,
      ]
    })
    return notes.length === 0 ? result : { ...result, context: [...(result.context ?? []), ...notes] }
  })

  // Deploying is the person's call: a push the model starts goes to them.
  on('tool.check', { tool: 'Bash' }, async ($, e, next) => {
    const command = (e.input as { command?: unknown }).command
    if (
      typeof command !== 'string' ||
      !/\bwmill\b[^|;&]*\bpush\b/.test(command) ||
      /--dry-run/.test(command)
    ) {
      return next(e)
    }
    const decision = await next(e)
    if (decision.decision === 'deny') return decision
    const { target } = await read($, status)
    const where = target ? `${target.workspaceId} on ${target.remote}` : 'the Windmill workspace'
    return { decision: 'ask', reason: `wmill-sync: this deploys to ${where}.` }
  })

  on('ui.render', { component: 'AbovePrompt' }, async ($, e, next) => {
    if (e.props.hasSurvey) return next(e)
    const current = await read($, status)
    const running = await read($, busy)
    const git = current.gitSync
    const hasGitWork = git !== null && (git.ahead > 0 || git.behind > 0)
    const isVisible = current.phase === 'error' || current.items.length > 0 || running !== null || hasGitWork
    if (current.phase === 'inactive' || !isVisible) return next(e)

    const { Box, Button, Text } = $.ui.resolve(e)
    const expanded = await read($, isExpanded)
    const where = current.target ? `${current.target.workspaceId}` : 'workspace'
    const toPush = current.items.filter(item => item.direction === 'push')
    const toPull = current.items.filter(item => item.direction === 'pull')
    const canPushItems = !git?.deployOnPush

    const header = (
      <Box key="header" flexDirection="row" gap={1}>
        <Text bold>wmill</Text>
        <Text dimColor>{where}</Text>
        {current.phase === 'error' ? (
          <Box flexShrink={1}>
            <Text color="red" wrap="truncate-end">
              {current.error}
            </Text>
          </Box>
        ) : (
          <Text>{summarize(current.items)}</Text>
        )}
        {running !== null && <Text dimColor>{`${running}…`}</Text>}
        {running === null && current.phase === 'checking' && <Text dimColor>checking…</Text>}
        {running === null && toPush.length > 0 && canPushItems && (
          <Button
            key="push-all"
            variant="primary"
            label={`Push ${toPush.length}`}
            onPress={() => push($, toPush)}
          />
        )}
        {running === null && toPull.length > 0 && (
          <Button key="pull-all" label={`Pull ${toPull.length}`} onPress={() => pull($, toPull)} />
        )}
        <Button key="refresh" label="Refresh" dimColor onPress={() => refresh($, { isFetchingGit: true })} />
        {current.items.length > 0 && (
          <Button
            key="toggle"
            label={expanded ? 'Hide' : 'Details'}
            dimColor
            onPress={() => update($, isExpanded, value => !value)}
          />
        )}
      </Box>
    )

    const gitRow = git && hasGitWork && (
      <Box key="git" flexDirection="row" gap={1}>
        <Text dimColor>{`  git ${git.branch}`}</Text>
        <Text>{`${git.ahead} ahead, ${git.behind} behind ${git.upstream ?? 'its upstream'}`}</Text>
        <Text dimColor>{git.deployOnPush ? '(Git Sync deploys on push)' : '(Git Sync)'}</Text>
        {running === null && git.behind > 0 && (
          <Button
            key="git-pull"
            label="git pull"
            onPress={() => act($, 'git pull', ['git', 'pull', '--ff-only'])}
          />
        )}
        {running === null && git.ahead > 0 && (
          <Button key="git-push" label="git push" onPress={() => act($, 'git push', ['git', 'push'])} />
        )}
      </Box>
    )

    // Rows are laid out by hand: the band narrows when a pane docks beside
    // it, and shrinking Text elements wrap mid-word.
    const buttonsWidth = 20
    const room = Math.max(20, e.props.bodyColumns - buttonsWidth - 14)
    const pathWidth = Math.min(
      Math.max(...current.items.map(item => item.path.length)),
      Math.ceil(room * 0.55),
    )
    const whyWidth = Math.min(Math.max(...current.items.map(item => item.why.length)), room - pathWidth)
    const rows =
      expanded &&
      current.items.map(item => (
        <Box key={`row-${item.key}`} flexDirection="row" gap={1}>
          <Text color={COLOR[item.direction]}>{` ${MARK[item.direction]}`}</Text>
          <Text dimColor>{KIND_LABEL[item.kind].slice(0, 8).padEnd(8)}</Text>
          <Text>{fit(item.path, pathWidth, 'middle')}</Text>
          <Text dimColor>{fit(item.why, whyWidth, 'end')}</Text>
          {running === null && item.direction !== 'pull' && item.direction !== 'conflict' && canPushItems && (
            <Button key={`push-${item.key}`} label="Push" onPress={() => push($, [item])} />
          )}
          {running === null && item.direction !== 'push' && item.direction !== 'conflict' && (
            <Button key={`pull-${item.key}`} label="Pull" onPress={() => pull($, [item])} />
          )}
          {item.direction === 'conflict' ? (
            <Button key={`resolve-${item.key}`} label="Resolve" onPress={() => showDiff($, item)} />
          ) : (
            <Button key={`diff-${item.key}`} label="Diff" dimColor onPress={() => showDiff($, item)} />
          )}
        </Box>
      ))

    return (
      <Box flexDirection="column">
        {header}
        {gitRow}
        {rows}
      </Box>
    )
  })

  on('ui.render', { component: 'Pane', requestId: DIFF_PANE }, async ($, e) => {
    const { Box, Button, Code, Text } = $.ui.resolve(e)
    const view = await read($, diffView)
    if (!view?.item) return <Text dimColor>No diff loaded.</Text>
    const running = await read($, busy)
    const { item } = view
    const isConflict = item.direction === 'conflict'
    const actions = running === null && (
      <Box key="actions" flexDirection="row" gap={1} flexWrap="wrap">
        {isConflict && (
          <Button
            key="merge"
            variant="primary"
            label="Ask Claude to merge"
            onPress={async () => {
              await closeDiff($)
              await askToMerge($, item)
            }}
          />
        )}
        {item.direction !== 'pull' && (
          <Button
            key="keep-local"
            label={isConflict ? 'Keep local (push)' : 'Push'}
            onPress={async () => {
              await closeDiff($)
              await push($, [item])
            }}
          />
        )}
        {item.direction !== 'push' && (
          <Button
            key="keep-remote"
            label={isConflict ? 'Keep workspace (pull)' : 'Pull'}
            onPress={async () => {
              await closeDiff($)
              await pull($, [item])
            }}
          />
        )}
      </Box>
    )
    return (
      <Box flexDirection="column" gap={1}>
        <Text bold>{view.title}</Text>
        {isConflict && (
          <Text dimColor>
            Both sides changed. Merge the workspace changes in, then keep local to push the result.
          </Text>
        )}
        {actions}
        {view.error !== null && <Text dimColor>{view.error}</Text>}
        {view.files.map(file => (
          <Box key={`file-${file.file}`} flexDirection="column">
            <Text dimColor>{file.isTruncated ? `${file.file} (cut to fit)` : file.file}</Text>
            <Code source={file.diff} format={file.isTruncated ? 'source' : 'diff'} path={file.file} />
          </Box>
        ))}
      </Box>
    )
  })
}
