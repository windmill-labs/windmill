export function randomId(): string {
  const c = globalThis.crypto
  if (c?.randomUUID) return c.randomUUID()
  // `randomUUID` needs a secure context; a plain http dev origin has `getRandomValues` only.
  const bytes = new Uint8Array(16)
  c.getRandomValues(bytes)
  bytes[6] = (bytes[6] & 0x0f) | 0x40
  bytes[8] = (bytes[8] & 0x3f) | 0x80
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`
}

export function now(): string {
  return new Date().toISOString()
}

/** Same rule as the server: the first message, cut to 25 characters. */
export function conversationTitle(firstMessage: string): string {
  const chars = Array.from(firstMessage)
  return chars.length > 25 ? `${chars.slice(0, 25).join('')}...` : firstMessage
}

export function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) return reject(abortError())
    const onAbort = () => {
      clearTimeout(timer)
      reject(abortError())
    }
    const timer = setTimeout(() => {
      signal?.removeEventListener('abort', onAbort)
      resolve()
    }, ms)
    signal?.addEventListener('abort', onAbort, { once: true })
  })
}

export function abortError(): Error {
  return new DOMException('The operation was aborted', 'AbortError')
}

export function isAbortError(e: unknown): boolean {
  return e instanceof Error && e.name === 'AbortError'
}

/**
 * The text a chat shows for a flow result, following what Windmill persists as the
 * assistant message when the last step is not an AI agent: `windmill_chat_answer`
 * when the result carries one (null means no message), an agent result's `output`,
 * a string as is, anything else as JSON.
 */
export function extractChatAnswer(result: unknown): string | undefined {
  if (result === null || result === undefined) return undefined
  if (typeof result === 'string') return result
  if (typeof result === 'object' && !Array.isArray(result)) {
    const obj = result as Record<string, unknown>
    if ('windmill_chat_answer' in obj) return formatAnswer(obj.windmill_chat_answer)
    if ('output' in obj && Array.isArray(obj.messages)) return formatAnswer(obj.output)
  }
  return JSON.stringify(result, null, 2)
}

function formatAnswer(value: unknown): string | undefined {
  if (value === null || value === undefined) return undefined
  return typeof value === 'string' ? value : JSON.stringify(value, null, 2)
}

/** A completed job whose result is Windmill's error envelope. */
export function isErrorResult(result: unknown): result is { error: Record<string, unknown> } {
  return (
    typeof result === 'object' &&
    result !== null &&
    'error' in result &&
    typeof (result as { error: unknown }).error === 'object' &&
    (result as { error: unknown }).error !== null
  )
}

export function errorResultMessage(result: { error: Record<string, unknown> }): string {
  const { message, name } = result.error
  if (typeof message === 'string' && message) {
    return typeof name === 'string' && name && name !== 'Error' ? `${name}: ${message}` : message
  }
  return JSON.stringify(result.error, null, 2)
}
