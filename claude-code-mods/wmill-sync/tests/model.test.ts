import { expect, test } from 'claude-code/testing'

import {
  compare,
  itemOfFile,
  parseShowDiffs,
  parseWorkspaceBindings,
  syncedByCommand,
  touchedItems,
  type LocalItem,
} from '../hooks/model'

const local = (kind: 'script' | 'flow' | 'variable', path: string, files: string[], hash: string) =>
  [`${kind}:${path}`, { ref: { kind, path }, files, hash }] as [string, LocalItem]

test('maps wmill files to the item they belong to', async () => {
  expect(itemOfFile('f/demo/greet.ts')).toEqual({ kind: 'script', path: 'f/demo/greet' })
  expect(itemOfFile('f/demo/greet.script.yaml')).toEqual({ kind: 'script', path: 'f/demo/greet' })
  expect(itemOfFile('f/demo/q.pg.sql')).toEqual({ kind: 'script', path: 'f/demo/q' })
  expect(itemOfFile('f/demo/pipeline__flow/a.ts')).toEqual({ kind: 'flow', path: 'f/demo/pipeline' })
  expect(itemOfFile('f/demo/dash.app/app.yaml')).toEqual({ kind: 'app', path: 'f/demo/dash' })
  expect(itemOfFile('f/demo/greeting.variable.yaml')).toEqual({ kind: 'variable', path: 'f/demo/greeting' })
  expect(itemOfFile('f/demo/hook.http_trigger.yaml')).toEqual({ kind: 'trigger', path: 'f/demo/hook' })
  expect(itemOfFile('f/demo/folder.meta.yaml')).toEqual({ kind: 'folder', path: 'f/demo' })
  expect(itemOfFile('wmill.yaml')).toBeUndefined()
  expect(itemOfFile('f/demo/node_modules/x.ts')).toBeUndefined()
})

test('reads the workspace bindings of a wmill.yaml', async () => {
  const yaml = [
    'includes:',
    '  - "f/**"',
    'workspaces:',
    '  main:',
    '    baseUrl: https://app.windmill.dev # prod',
    '    workspaceId: acme',
    '  staging:',
    '    gitBranch: develop',
    '    overrides:',
    '      skipSecrets: false',
    'codebases: []',
  ].join('\n')
  expect(parseWorkspaceBindings(yaml)).toEqual([
    { name: 'main', baseUrl: 'https://app.windmill.dev', workspaceId: 'acme' },
    { name: 'staging', gitBranch: 'develop' },
  ])
  expect(parseWorkspaceBindings('workspaces:  {}\n')).toEqual([])
})

test('tells push, pull and conflict apart from the last-synced record', async () => {
  const base = {
    'script:f/a': { localHash: 'a1', version: 'v1' },
    'script:f/b': { localHash: 'b1', version: 'v1' },
    'script:f/c': { localHash: 'c1', version: 'v1' },
    'variable:f/d': { localHash: 'd1', version: null },
  }
  const { items, nextBase } = compare({
    changes: [
      { type: 'edited', path: 'f/a.ts' },
      { type: 'edited', path: 'f/b.ts' },
      { type: 'edited', path: 'f/c.ts' },
      { type: 'edited', path: 'f/d.variable.yaml' },
      { type: 'added', path: 'f/e.ts' },
    ],
    local: new Map([
      local('script', 'f/a', ['f/a.ts'], 'a2'),
      local('script', 'f/b', ['f/b.ts'], 'b1'),
      local('script', 'f/c', ['f/c.ts'], 'c2'),
      local('variable', 'f/d', ['f/d.variable.yaml'], 'd1'),
      local('script', 'f/same', ['f/same.ts'], 's1'),
    ]),
    versions: new Map([
      ['script:f/a', 'v1'],
      ['script:f/b', 'v2'],
      ['script:f/c', 'v2'],
      ['script:f/e', 'v1'],
      ['script:f/same', 'v9'],
    ]),
    base,
    gitDirty: new Set(),
  })
  const byPath = Object.fromEntries(items.map(item => [item.path, item.direction]))
  expect(byPath).toEqual({ 'f/a': 'push', 'f/b': 'pull', 'f/c': 'conflict', 'f/d': 'pull', 'f/e': 'pull' })
  // An item that matches is recorded as synced at its current hash and version.
  expect(nextBase['script:f/same']).toEqual({ localHash: 's1', version: 'v9' })
  // A differing item keeps its old record until it matches again.
  expect(nextBase['script:f/a']).toEqual(base['script:f/a'])
})

test('falls back to git when nothing was recorded', async () => {
  const { items } = compare({
    changes: [
      { type: 'edited', path: 'f/mine.ts' },
      { type: 'edited', path: 'f/theirs.ts' },
      { type: 'deleted', path: 'f/new.ts' },
    ],
    local: new Map([
      local('script', 'f/mine', ['f/mine.ts'], 'm'),
      local('script', 'f/theirs', ['f/theirs.ts'], 't'),
      local('script', 'f/new', ['f/new.ts'], 'n'),
    ]),
    versions: new Map(),
    base: {},
    gitDirty: new Set(['f/mine.ts', 'f/new.ts']),
  })
  const byPath = Object.fromEntries(items.map(item => [item.path, `${item.direction}: ${item.why}`]))
  expect(byPath).toEqual({
    'f/mine': 'push: edited locally',
    'f/new': 'push: new locally',
    'f/theirs': 'pull: changed on the workspace',
  })
})

test("turns the CLI's coloured --show-diffs output into a unified hunk", async () => {
  const raw =
    '\u001b[34m\u001b[90mComputing the files to update locally to match remote\u001b[39m\u001b[39m\n' +
    '\u001b[34m\u001b[33m~ flow f/demo/pipeline__flow/b.ts\u001b[90m\u001b[33m\u001b[39m\u001b[39m\n' +
    '\u001b[34m\u001b[37mexport async function main(x: number) {\n' +
    '\u001b[0m\u001b[31m  return x + 2\n' +
    '\u001b[0m\u001b[32m  return x + 1\n' +
    '\u001b[0m\u001b[37m}\n' +
    '\u001b[0m\u001b[39m\n' +
    '\u001b[34m\u001b[90mDry run complete.\u001b[39m\u001b[39m\n'
  expect(parseShowDiffs(raw)).toEqual([
    {
      file: 'f/demo/pipeline__flow/b.ts',
      isTruncated: false,
      diff: [
        '--- local/f/demo/pipeline__flow/b.ts',
        '+++ workspace/f/demo/pipeline__flow/b.ts',
        '@@ -1,3 +1,3 @@',
        ' export async function main(x: number) {',
        '-  return x + 2',
        '+  return x + 1',
        ' }',
      ].join('\n'),
    },
  ])
})

test('finds the items a tool call names, from a file path or a shell command', async () => {
  const root = '/work/project'
  expect(touchedItems(root, { file_path: '/work/project/f/demo/greet.ts' })).toEqual([
    { kind: 'script', path: 'f/demo/greet' },
  ])
  expect(touchedItems(root, { file_path: '/elsewhere/f/demo/greet.ts' })).toEqual([])
  expect(
    touchedItems(root, {
      command: `sed -i '' 's/a/b/' f/demo/greet.ts && cat "./f/demo/pipeline__flow/a.ts" /work/project/f/demo/sum.py`,
    }),
  ).toEqual([
    { kind: 'script', path: 'f/demo/greet' },
    { kind: 'flow', path: 'f/demo/pipeline' },
    { kind: 'script', path: 'f/demo/sum' },
  ])
  expect(touchedItems(root, { command: 'git status' })).toEqual([])
})

test('tells which items a shell command synced', async () => {
  expect(
    syncedByCommand('wmill sync pull --yes -i f/demo/greet.ts && sed -i "" s/a/b/ f/demo/greet.ts'),
  ).toEqual([{ kind: 'script', path: 'f/demo/greet' }])
  expect(syncedByCommand('wmill sync push --yes')).toBe('all')
  expect(syncedByCommand('wmill sync pull --dry-run --json-output')).toBeUndefined()
  expect(syncedByCommand('wmill flow push f/demo/pipeline__flow f/demo/pipeline')).toEqual([
    { kind: 'flow', path: 'f/demo/pipeline' },
  ])
  expect(syncedByCommand('git status')).toBeUndefined()
})
