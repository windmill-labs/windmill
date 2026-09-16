import type { ChatOptions, FetchLike, HistoryMode, StorageLike, TokenSource } from './types'

export interface ResolvedConfig {
  flowPath: string
  baseUrl: string
  workspace: string
  token: TokenSource | undefined
  history: HistoryMode
  /** The caller chose `history`; a failing server history is then an error, not a fallback. */
  historyExplicit: boolean
  inputs: Record<string, unknown>
  fetch: FetchLike | undefined
  storage: StorageLike | undefined
  storageKey: string | undefined
  pageSize: number
  pollDelayMs: number | undefined
  conversationKind: 'test' | 'deployed'
  run: ChatOptions['run']
  onFinish: ChatOptions['onFinish']
  onError: ChatOptions['onError']
}

export interface RawAppContext {
  baseUrl: string
  workspace: string
  /** The viewer's SDK token in a sandboxed raw app; the session cookie otherwise. */
  token?: string
}

/**
 * The Windmill a raw app bundle runs in. A sandboxed app gets `window.process.env`
 * from its wrapper once the viewer consented to the declared SDK scopes; an
 * unsandboxed one runs on the Windmill origin with the viewer's session and only
 * has `window.ctx`.
 */
export function detectRawApp(): RawAppContext | undefined {
  const g = globalThis as {
    process?: { env?: Record<string, string | undefined> }
    ctx?: { workspace?: unknown }
    location?: { origin?: string }
  }
  const env = g.process?.env
  if (env?.WM_RAW_APP === 'true' && env.WM_TOKEN && env.BASE_URL && env.WM_WORKSPACE) {
    return { baseUrl: env.BASE_URL, workspace: env.WM_WORKSPACE, token: env.WM_TOKEN }
  }
  const workspace = g.ctx?.workspace
  const origin = g.location?.origin
  if (typeof workspace === 'string' && workspace && origin && origin !== 'null') {
    return { baseUrl: origin, workspace }
  }
  return undefined
}

export function resolveConfig(options: ChatOptions): ResolvedConfig {
  if (!options.flowPath) throw new Error('windmill-chat: flowPath is required')
  const explicitToken = options.token !== undefined
  const detected =
    options.baseUrl && options.workspace && explicitToken ? undefined : detectRawApp()
  const baseUrl = options.baseUrl ?? detected?.baseUrl
  const workspace = options.workspace ?? detected?.workspace
  if (!baseUrl || !workspace) {
    throw new Error(
      'windmill-chat: pass baseUrl and workspace. They are only detected inside a raw app: an unsandboxed one on the Windmill origin, or a sandboxed one whose policy declares frontend SDK scopes.'
    )
  }
  // The raw app's token belongs to its own Windmill; it never travels to another origin.
  const token =
    options.token ?? (options.baseUrl === undefined || options.baseUrl === detected?.baseUrl ? detected?.token : undefined)
  return {
    flowPath: options.flowPath,
    baseUrl,
    workspace,
    token,
    history: options.history ?? (explicitToken ? 'local' : 'server'),
    historyExplicit: options.history !== undefined,
    inputs: options.inputs ?? {},
    fetch: options.fetch,
    storage: options.storage,
    storageKey: options.storageKey,
    pageSize: options.pageSize ?? 50,
    pollDelayMs: options.pollDelayMs,
    conversationKind: options.conversationKind ?? 'deployed',
    run: options.run,
    onFinish: options.onFinish,
    onError: options.onError
  }
}
