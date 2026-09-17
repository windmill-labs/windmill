# windmill-chat

Build a chat interface on a Windmill flow deployed in **chat mode**, from any frontend
or from a Windmill raw app. The library is headless: it runs the flow, follows the
answer as it streams, keeps the conversation history, and hands you state to render.

```
npm install windmill-chat
```

No runtime dependencies. Optional peers: `react` for `windmill-chat/react`, `ai` for
`windmill-chat/ai-sdk`, `@assistant-ui/react` for `windmill-chat/assistant-ui`.

| You build the UI with | Import | You get |
|---|---|---|
| Vercel AI SDK `useChat`, AI Elements | `windmill-chat/ai-sdk` | a `ChatTransport`: `useChat({ transport })`, nothing else changes |
| assistant-ui | `windmill-chat/assistant-ui` | a runtime for `AssistantRuntimeProvider`, threads included |
| Your own components | `windmill-chat/react` or `windmill-chat` | a hook / a store with messages, status and actions |

## The flow

Any deployed flow with **Chat mode** enabled in its settings works. Windmill passes the
message as the `user_message` input and threads the conversation through `memory_id`,
so an AI agent step remembers earlier turns. The answer is:

- what the last step streams, when it is an AI agent step;
- otherwise the flow's result: its `windmill_chat_answer` field when it has one, a
  string as is, anything else as JSON.

## Vercel AI SDK (`useChat`, AI Elements)

```tsx
import { useChat } from '@ai-sdk/react'
import { createWindmillChatTransport } from 'windmill-chat/ai-sdk'

const transport = createWindmillChatTransport({
  baseUrl: 'https://app.windmill.dev',
  workspace: 'acme',
  flowPath: 'f/support/assistant',
  token: () => fetch('/api/windmill-token').then((r) => r.text())
})

export function Support() {
  const { messages, status, sendMessage, stop } = useChat({ id: conversationId, transport })
  // render `messages[i].parts`: text, reasoning and dynamic-tool parts, as with any AI SDK backend
}
```

The chat `id` is the conversation: reuse it to continue one, and pass a UUID when you
also read server history, so it matches what `flow_conversations` stores (any other id
maps to a fixed UUID). `sendMessage(msg, { body })` sends extra flow inputs. Tool calls
arrive as `dynamic-tool` parts (`input-available → output-available | output-error`),
which AI Elements' `<Tool>` renders as is. A failed flow surfaces as `error`.
`regenerate()` runs the flow again with the same message: a new turn on the server,
not a replacement of the previous answer.

The transport also carries the history helpers: `transport.loadMessages(id)` returns
`UIMessage`s for `useChat({ messages })` or `setMessages`, `transport.listConversations()`
and `transport.deleteConversation(id)`. A loaded user message lists the files it carried
in `metadata.attachments`; `WindmillChatApi.attachmentUrl` gives each one's download URL.
Sending attachments is not supported: `sendMessage` with `files` is refused with an
explanatory error.

## assistant-ui

```tsx
import { AssistantRuntimeProvider } from '@assistant-ui/react'
import { useWindmillRuntime } from 'windmill-chat/assistant-ui'

export function Support() {
  const runtime = useWindmillRuntime({ baseUrl, workspace, flowPath, token })
  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <Thread /> {/* your assistant-ui components, thread list included */}
    </AssistantRuntimeProvider>
  )
}
```

Conversations are threads: `ThreadListPrimitive` switches, creates and deletes them.
Tool calls render through your `tools` components (`MessagePrimitive.Parts`), reasoning
through `Reasoning`. It takes the same options as `useWindmillChat` below.

## React

```tsx
import { useWindmillChat } from 'windmill-chat/react'

export function Support() {
  const chat = useWindmillChat({
    baseUrl: 'https://app.windmill.dev',
    workspace: 'acme',
    flowPath: 'f/support/assistant',
    token: () => fetch('/api/windmill-token').then((r) => r.text())
  })
  const [draft, setDraft] = useState('')

  return (
    <div>
      {chat.messages.map((m) => (
        <p key={m.id} data-role={m.role} data-pending={m.pending}>
          {m.content}
        </p>
      ))}
      <form
        onSubmit={(e) => {
          e.preventDefault()
          chat.sendMessage(draft)
          setDraft('')
        }}
      >
        <input value={draft} onChange={(e) => setDraft(e.target.value)} />
        <button disabled={chat.status !== 'idle' && chat.status !== 'error'}>Send</button>
        {chat.status === 'streaming' && (
          <button type="button" onClick={chat.stop}>
            Stop
          </button>
        )}
      </form>
    </div>
  )
}
```

The hook returns the [state](#state) plus the chat's methods. It recreates the chat
(fresh state, old one destroyed) when `flowPath`, `baseUrl`, `workspace`, `history`,
`storageKey` or the credential change: a different token string, or a switch between
no token, a string and a function; and when a `run` callback appears or goes away.
A token function is called through a ref, so passing a new closure on every render
is fine and never resets the chat, and so are `run` and the callbacks; when users
sign in and out behind a token function, change `storageKey` (their id) so local
history and state start over with them.

## Raw apps

Inside a Windmill raw app nothing needs configuring: the chat runs as the viewer,
against the Windmill the app is served from.

```tsx
const chat = useWindmillChat({ flowPath: 'f/support/assistant' })
```

- **Unsandboxed app** (the default): the viewer's session is used. Viewers need
  permission to run the flow.
- **Sandboxed app**: declare `jobs:run` in the app's frontend SDK scopes, and
  `flow_conversations:write` for server-side history. The viewer consents once and
  the app receives a token restricted to those scopes.
- **`wmill app dev`**: there is no viewer session on the dev server, so pass
  `baseUrl`, `workspace` and `token` explicitly during development.

## Any framework

`createChat` returns a store: `subscribe` calls the listener immediately and on every
change, and returns the unsubscribe function. That is the Svelte store contract, so
`$chat` works as is; other frameworks wrap it in a few lines.

```ts
import { createChat } from 'windmill-chat'

const chat = createChat({ baseUrl, workspace, flowPath, token })
chat.subscribe((state) => render(state))
await chat.sendMessage('Hello')
```

```svelte
<script>
  import { createChat } from 'windmill-chat'
  const chat = createChat({ flowPath: 'f/support/assistant' })
</script>

{#each $chat.messages as m (m.id)}
  <p>{m.content}</p>
{/each}
<button onclick={() => chat.sendMessage(draft)}>Send</button>
```

## Options

| Option | |
|---|---|
| `flowPath` | Path of the deployed flow, e.g. `f/support/assistant`. Required. |
| `baseUrl` | The Windmill origin. Detected inside a raw app. |
| `workspace` | Detected inside a raw app. |
| `token` | A token, or a function returning one (called before every request, so it can fetch a short-lived token from your backend). Omit it inside a raw app. |
| `history` | `'server'`, `'local'` or `'none'`, see [History](#history). Defaults to `'server'` with a viewer session and `'local'` with an explicit `token`. |
| `inputs` | Extra flow inputs sent with every message. `sendMessage(text, { inputs })` adds per-message ones. |
| `storageKey` | Namespace for `local` history, e.g. the signed-in user's id. Local history is per browser and per flow; without it, users sharing a browser share it. |
| `fetch`, `storage` | Replacements for the globals, for tests and unusual runtimes. |
| `pageSize` | Messages and conversations per page of server history. Default 50. |
| `pollDelayMs` | How often, in ms, the server polls a running turn for the stream (Enterprise; 50 at the fastest, other servers ignore it). Unset, the server relaxes from 100 ms to 3 s over a long turn; set it when tokens must keep flowing at that pace. |
| `onFinish`, `onError` | Called when a turn has its answer, or could not run at all. |
| `run` | Runs the flow for a turn yourself and returns the job id, instead of the deployed flow at `flowPath` (Windmill's editor chats with an undeployed flow through a preview run this way). Pass `memory_id` = the conversation id. |

## State

```ts
interface ChatState {
  conversationId: string | undefined
  messages: ChatMessage[]
  status: 'idle' | 'submitted' | 'streaming' | 'error'
  error: Error | undefined
  conversations: Conversation[]
  history: 'server' | 'local' | 'none'
  loadingMessages: boolean
  hasMoreMessages: boolean
}

interface ChatMessage {
  id: string
  role: 'user' | 'assistant' | 'tool' | 'system'
  content: string
  reasoning?: string // the model's reasoning summary, when streamed
  tool?: { callId?: string; name: string; arguments?: string; result?: string; status: 'running' | 'success' | 'error' }
  success: boolean // false for a failed flow or tool
  pending: boolean // still streaming, or not yet confirmed by the server
  createdAt: string
  jobId?: string
  stepName?: string
  serverId?: string // the persisted row; `id` itself never changes, so list keys are stable
}
```

A turn goes `submitted` (the flow is queued) → `streaming` (the answer is arriving) →
`idle`. Tool calls appear as `tool` messages whose `status` moves from `running` to
`success` or `error`. A flow that fails still completes the turn: its error is the
answer, an `assistant` message with `success: false`. `status: 'error'` (with `error`
set) means the turn could not run or be followed at all, such as a refused request.

Methods: `sendMessage(text, { inputs? })`, `stop()`, `newConversation()`,
`selectConversation(id)`, `loadConversations({ page?, perPage? })`,
`deleteConversation(id)`, `loadOlderMessages()`, `destroy()`. Switching conversations
stops following the current answer; the flow keeps running and, with server history,
its answer is there when you come back.

## History

Windmill stores every conversation of a chat-mode flow, and each Windmill user sees
only their own. `history: 'server'` reads that store: `loadConversations()` lists
them, `selectConversation(id)` loads one, `loadOlderMessages()` pages back. Every
message rendered from the server carries its `jobId` and `stepName`.

That store is keyed by the **Windmill user**, so it fits a viewer session or a token
issued per user. With one token shared by every visitor of a site, all visitors would
see each other's conversations. For that setup use `history: 'local'` (the default
with an explicit `token`): the conversation list and messages stay in the browser's
`localStorage`, per Windmill instance, workspace and flow. `'none'` keeps nothing
beyond the page.

When the default `'server'` mode turns out unreadable (a token or sandboxed app
without `flow_conversations` scopes), the chat switches itself to `'local'` and
`state.history` says so. Passing `history` explicitly disables that fallback.

## Tokens

Anything a browser holds can be read by its user, so give a chat token exactly what
the chat needs:

| Setup | Scopes |
|---|---|
| Public site, one token for everyone | `jobs:run:flows:f/support/assistant`, and `history: 'local'`. The token can run that one flow and follow its jobs, nothing else. |
| Per-user tokens minted by your backend | The above plus `flow_conversations:write`, with `history: 'server'` (an explicit token defaults to local history). Return them from an endpoint and pass `token: () => fetch(...)`. |
| A Windmill user in the browser (raw app, embedded Windmill) | No token: the session is used. |

The token's user must be allowed to run the flow. `stop()` closes the stream in any
case; cancelling the run on the server as well needs `jobs:write`, which also lets the
token read every job its user can see, so leave it out unless that matters.

Anyone holding the token can run the flow with inputs of their choosing, so a flow
exposed this way should treat `user_message` and the other inputs as untrusted.

## Lower level

`WindmillChatApi` wraps the endpoints (`runFlow`, `streamJob`, `listConversations`,
`listMessages`, `deleteConversation`, `cancelJob`), `followJob` follows a run to
completion across the server's stream timeouts, `parseStreamEvents` decodes the AI
agent stream, `extractChatAnswer` turns a flow result into the text a chat shows, and
`conversationIdFor` maps any chat id to its conversation UUID. They are exported for
custom integrations.

## For AI coding agents

When asked to add a chat over a Windmill flow: the flow must be deployed with chat mode
on. Pick the entry point from the table at the top (`useChat` → `windmill-chat/ai-sdk`,
assistant-ui → `windmill-chat/assistant-ui`, otherwise `windmill-chat/react`). Inside a
Windmill raw app pass only `flowPath`. Elsewhere pass `baseUrl`, `workspace` and a
`token`; for a public page use a token scoped to `jobs:run:flows:<flowPath>` and leave
`history` at its default. Render `role`, `content`, `pending`, `success` and
`tool.status`; never build the SSE handling yourself.
