/**
 * The code a webhook panel offers to copy beyond its REST, Curl and Fetch tabs: the chat SDK for
 * what keeps a conversation, a brief for a coding assistant, and an agent's own Curl and Fetch.
 * Every example is meant to run as copied once the token is filled in, so each one is exercised
 * against a live script, flow or agent before it changes here.
 */
export type WebhookTarget = {
	/** `https://host/api/w/<workspace>`, base path included. */
	api: string
	kind: 'script' | 'flow' | 'agent'
	path: string
	/** Whether it keeps the conversation: an agent with managed memory, a flow in chat mode. Only
	 *  then is `memory_id` worth sending. */
	chat: boolean
	token: string
	/** An example of its inputs, for a script or a flow outside chat mode. */
	inputs?: Record<string, unknown>
}

const noun = (t: WebhookTarget) => t.kind

/** The segment of `/jobs/run/...` that names the target by path. */
function runSegment({ kind, path }: WebhookTarget): string {
	return kind === 'agent' ? `agent/${path}` : kind === 'flow' ? `f/${path}` : `p/${path}`
}

export function runUrl(t: WebhookTarget): string {
	return `${t.api}/jobs/run/${runSegment(t)}`
}

/** The token scope that can run this target and nothing else. */
export function runScope({ kind, path }: WebhookTarget): string {
	return `jobs:run:${kind === 'agent' ? 'agents' : kind === 'flow' ? 'flows' : 'scripts'}:${path}`
}

/** What the conversations are filed under: an agent's under `<path>.chat`, a flow's under its path. */
function conversationsPath({ kind, path }: WebhookTarget): string {
	return kind === 'agent' ? `${path}.chat` : path
}

/** Agents only: Curl that runs the agent and polls for its answer. */
export function curlExample(t: WebhookTarget): string {
	const conversation = t.chat
		? `# Any string names a conversation: reuse it for follow-ups, pick a new one to start over.
CONVERSATION=$(uuidgen)
`
		: ''
	return `TOKEN='${t.token}'
${conversation}JOB=$(curl -s -X POST "${runUrl(t)}${t.chat ? '?memory_id=$CONVERSATION' : ''}" \\
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \\
  -d '{"user_message": "Hello"}')

while true; do
  RES=$(curl -s "${t.api}/jobs_u/completed/get_result_maybe/$JOB" -H "Authorization: Bearer $TOKEN")
  if [ "$(echo "$RES" | jq .completed)" = "true" ]; then
    echo "$RES" | jq -r '.result.output // .result.error.message'
    break
  fi
  sleep 1
done`
}

/** Agents only: Fetch that runs the agent and streams its answer. */
export function fetchExample(t: WebhookTarget): string {
	const usage = t.chat
		? `export async function main() {
  // Any string names a conversation: reuse it for follow-ups, pick a new one to start over.
  const conversation = crypto.randomUUID();
  await ask('Hello', conversation, (text) => console.log(text));
  return await ask('What did I just say?', conversation, (text) => console.log(text));
}`
		: `export async function main() {
  return await ask('Hello', (text) => console.log(text));
}`
	return `const API = '${t.api}';
const HEADERS = { Authorization: 'Bearer ${t.token}' };

${usage}

/** Runs the agent and streams its answer to \`onText\` as it is written; resolves to the full answer. */
async function ask(message${t.chat ? ', conversation' : ''}, onText) {
  const run = await fetch(\`\${API}/jobs/run/agent/${t.path}${t.chat ? '?memory_id=${conversation}' : ''}\`, {
    method: 'POST',
    headers: { ...HEADERS, 'Content-Type': 'application/json' },
    body: JSON.stringify({ user_message: message })
  });
  if (!run.ok) throw new Error(await run.text());
  const job = (await run.text()).trim();

  // The server closes the stream now and then: reconnect from the last offset, never rerun.
  let offset = 0;
  let pending = '';
  for (;;) {
    const res = await fetch(
      \`\${API}/jobs_u/getupdate_sse/\${job}?fast=true&only_result=true&stream_offset=\${offset}\`,
      { headers: HEADERS }
    );
    let buffer = '';
    for await (const chunk of res.body.pipeThrough(new TextDecoderStream())) {
      buffer += chunk;
      let end;
      while ((end = buffer.indexOf('\\n\\n')) >= 0) {
        const frame = buffer.slice(0, end);
        buffer = buffer.slice(end + 2);
        if (!frame.startsWith('data:')) continue;
        const update = JSON.parse(frame.slice(5));
        if (update.new_result_stream) {
          // One JSON event per line; a chunk can stop mid-line.
          pending += update.new_result_stream;
          offset = update.stream_offset;
          const lines = pending.split('\\n');
          pending = lines.pop();
          for (const line of lines) {
            const event = JSON.parse(line);
            if (event.type === 'token_delta') onText(event.content);
            // Also: reasoning_token_delta, tool_call, tool_call_arguments, tool_execution, tool_result
          }
        }
        if (update.completed) {
          const result = update.only_result;
          if (result?.error) throw new Error(result.error.message);
          return result?.output;
        }
      }
    }
  }
}`
}

function sdkOptions(t: WebhookTarget, indent: string): string {
	const connection = `  baseUrl: '${t.api.replace(/\/api\/w\/[^/]+$/, '')}',
  workspace: '${t.api.split('/').pop()}',
  token: TOKEN,`
	if (t.kind !== 'agent') {
		return `{
${connection}
  // With a token, history stays in this browser; history: 'server' also needs the token's
  // flow_conversations:write scope.
  flowPath: '${t.path}'
}`.replaceAll('\n', '\n' + indent)
	}
	return `{
${connection}
  // What the agent's conversations are filed under. With a token, history stays in this
  // browser; history: 'server' also needs the token's flow_conversations:write scope.
  flowPath: '${t.path}.chat',
  // Runs the agent for each turn; the conversation id is its memory.
  run: async (args, { conversationId, signal }) => {
    const res = await fetch(\`\${API}/jobs/run/agent/${t.path}?memory_id=\${conversationId}\`, {
      method: 'POST',
      headers: { Authorization: \`Bearer \${TOKEN}\`, 'Content-Type': 'application/json' },
      body: JSON.stringify(args),
      signal
    })
    if (!res.ok) throw new Error(await res.text())
    return (await res.text()).trim()
  }
}`.replaceAll('\n', '\n' + indent)
}

/** The constants the SDK examples open with: an agent's `run` also needs the API's url. */
function sdkConstants(t: WebhookTarget): string {
	return `${t.kind === 'agent' ? `const API = '${t.api}'\n` : ''}const TOKEN = '${t.token}'`
}

/** A chat over an agent or a chat-mode flow, in React. */
export function sdkReactExample(t: WebhookTarget): string {
	return `import { useState } from 'react'
// On the Vercel AI SDK or assistant-ui? Use @windmill-labs/chat-sdk/ai-sdk or /assistant-ui instead:
// https://github.com/windmill-labs/windmill/tree/main/chat-sdk#readme
import { useWindmillChat } from '@windmill-labs/chat-sdk/react'

${sdkConstants(t)}

export function ${t.kind === 'agent' ? 'AgentChat' : 'FlowChat'}() {
  const chat = useWindmillChat(${sdkOptions(t, '  ')})
  const [draft, setDraft] = useState('')
  // Also chat.stop(), and the conversations: chat.conversations, chat.selectConversation(id), chat.newConversation()
  return (
    <div>
      {chat.messages.map((m) => <p key={m.id}><b>{m.role}</b> {m.content}</p>)}
      <form onSubmit={(e) => { e.preventDefault(); chat.sendMessage(draft); setDraft('') }}>
        <input value={draft} onChange={(e) => setDraft(e.target.value)} disabled={chat.status === 'submitted' || chat.status === 'streaming'} />
      </form>
    </div>
  )
}`
}

/** A chat over an agent or a chat-mode flow, with any framework or none. */
export function sdkExample(t: WebhookTarget): string {
	return `import { createChat } from '@windmill-labs/chat-sdk'

${sdkConstants(t)}

const chat = createChat(${sdkOptions(t, '')})

// Called now and on every change: messages stream in as they are written. This is the Svelte
// store contract, so in Svelte $chat works as is.
chat.subscribe(({ messages, status }) => console.log(status, messages.at(-1)?.content)) // your UI here
await chat.sendMessage('Hello')
// Also chat.stop(), and the conversations: chat.newConversation(), chat.selectConversation(id)`
}

function conversationSection(t: WebhookTarget): string {
	const sdkPointer =
		t.kind === 'agent'
			? `Point it at the
agent with \`flowPath: '${t.path}.chat'\` and a \`run(args, { conversationId, signal })\` option
that POSTs \`args\` to the run endpoint with \`memory_id=conversationId\` and returns the job id.`
			: `Point it at the
flow with \`flowPath: '${t.path}'\`: it runs the flow itself.`
	return `
## Conversations

The ${noun(t)} keeps the conversation. Pass \`memory_id\` as a query parameter on every run: any
string names a conversation, a UUID is best. Reuse it for follow-ups; a new one starts over.
Without it each message is answered on its own.

Windmill also keeps each conversation, as the token's owner's. One token shared by all your
users makes every conversation theirs, so keep your users' conversation ids in your own app.
Reading them back needs a token with the \`flow_conversations:read\` scope (\`:write\` to delete):
- \`GET ${t.api}/flow_conversations/list?flow_path=${conversationsPath(t)}\` (\`page\`, \`per_page\`).
  A conversation's \`id\` is the \`memory_id\` when that was a UUID.
- \`GET ${t.api}/flow_conversations/{id}/messages\`: its latest messages, oldest first.
- \`DELETE ${t.api}/flow_conversations/delete/{id}\`.

In TypeScript or JavaScript, prefer the \`@windmill-labs/chat-sdk\` package (npm, source at
https://github.com/windmill-labs/windmill/tree/main/chat-sdk): it does all of this, streaming included.
\`createChat\` from \`@windmill-labs/chat-sdk\` works with any framework (or none); use the adapter that
matches the project when there is one: \`useWindmillChat\` from \`@windmill-labs/chat-sdk/react\`, the Vercel
AI SDK transport (\`@windmill-labs/chat-sdk/ai-sdk\`) or the assistant-ui runtime
(\`@windmill-labs/chat-sdk/assistant-ui\`). ${sdkPointer}
`
}

function authSection(t: WebhookTarget): string {
	const specific = `${t.kind === 'agent' ? 'an\nagent' : `a\n${t.kind}`}-specific token (scope \`${runScope(t)}\`): that one can only run this ${noun(t)}`
	return `## Authentication

Every request sends \`Authorization: Bearer <token>\`. Read the token from the \`WINDMILL_TOKEN\`
environment variable (or the project's secret store under that name), never hardcode it: the ${noun(t)}
runs as the token's owner, with their permissions. Keep it on the server, unless it is ${specific}, so a
browser app may read it from the bundler's public variable (\`VITE_WINDMILL_TOKEN\`,
\`NEXT_PUBLIC_WINDMILL_TOKEN\`, ...).`
}

function agentPrompt(t: WebhookTarget): string {
	return `Integrate the Windmill AI agent \`${t.path}\` into this project.

${authSection(t)}

## Sending a message

\`POST ${runUrl(t)}\` with the JSON body \`{"user_message": "..."}\` starts a run and returns its
job id as plain text. An error is a non-2xx status with the reason as text.
${t.chat ? conversationSection(t) : ''}
## Streaming the answer

\`GET ${t.api}/jobs_u/getupdate_sse/{job_id}?fast=true&only_result=true\` is a server-sent events
stream of JSON objects:
- \`new_result_stream\`: text to append to a buffer holding one JSON event per line. A chunk can
  stop mid-line, so parse complete lines only. Event types: \`token_delta\` (\`content\`: answer text),
  \`reasoning_token_delta\`, \`tool_call\`, \`tool_call_arguments\`, \`tool_execution\`, \`tool_result\`
  (\`call_id\`, \`function_name\`, \`result\`, \`success\`).
- \`stream_offset\`: how far the stream has been read. The server closes the stream now and then;
  reconnect with \`&stream_offset=<last value>\` and never run the agent again for the same message.
- \`completed: true\` ends it. \`only_result.output\` is the full answer, or \`only_result.error.message\`
  when the run failed.

Without streaming, poll \`GET ${t.api}/jobs_u/completed/get_result_maybe/{job_id}\` until
\`completed\` is true; the answer is \`result.output\`.

To stop a run: \`POST ${t.api}/jobs_u/queue/cancel/{job_id}\` with \`{"reason": "..."}\`.
`
}

function runnablePrompt(t: WebhookTarget): string {
	const segment = runSegment(t)
	const body = t.chat
		? '`{"user_message": "..."}`'
		: `its inputs, for example \`${JSON.stringify(t.inputs ?? {})}\``
	return `Integrate the Windmill ${t.chat ? 'chat flow' : noun(t)} \`${t.path}\` into this project.

${authSection(t)}

## Running it

The JSON body of each request is ${body}. An error is a non-2xx status with the reason as text.
- \`POST ${t.api}/jobs/run/${segment}\` starts a run and returns its job id as plain text.
- \`POST ${t.api}/jobs/run_wait_result/${segment}\` runs it and answers with its result as JSON, for
  runs short enough to wait on.
- \`POST ${t.api}/jobs/run_and_stream/${segment}\` answers with a server-sent events stream of its
  updates, for a ${noun(t)} that streams its result.
${t.chat ? conversationSection(t) : ''}
## Reading the result of a started run

Poll \`GET ${t.api}/jobs_u/completed/get_result_maybe/{job_id}\` until \`completed\` is true. \`result\`
is what the ${noun(t)} returned${t.kind === 'flow' ? ' (a flow ending on an AI agent step answers in `result.output`)' : ''}, or carries \`result.error.message\` when the run failed.

To stop a run: \`POST ${t.api}/jobs_u/queue/cancel/{job_id}\` with \`{"reason": "..."}\`.
`
}

/** A brief for a coding assistant asked to call the target. It reads the token from `WINDMILL_TOKEN`. */
export function integrationPrompt(t: WebhookTarget): string {
	return t.kind === 'agent' ? agentPrompt(t) : runnablePrompt(t)
}
