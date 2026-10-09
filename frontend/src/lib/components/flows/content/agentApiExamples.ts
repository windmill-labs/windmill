/**
 * The code an agent's API tab offers to copy. Every example is meant to run as copied once the
 * token is filled in, so each one is exercised against a live agent before it changes here.
 */
export type AgentApiTarget = {
	/** `https://host/api/w/<workspace>`, base path included. */
	api: string
	agentPath: string
	/** Whether the agent keeps the conversation: only then is `memory_id` worth sending. */
	chat: boolean
	token: string
}

export function runUrl({ api, agentPath }: AgentApiTarget): string {
	return `${api}/jobs/run/agent/${agentPath}`
}

export function curlExample(t: AgentApiTarget): string {
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

export function fetchExample(t: AgentApiTarget): string {
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
  const run = await fetch(\`\${API}/jobs/run/agent/${t.agentPath}${t.chat ? '?memory_id=${conversation}' : ''}\`, {
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

export function sdkExample(t: AgentApiTarget): string {
	return `import { useWindmillChat } from 'windmill-chat/react'

const API = '${t.api}'
const TOKEN = '${t.token}'

export function AgentChat() {
  const chat = useWindmillChat({
    baseUrl: '${t.api.replace(/\/api\/w\/[^/]+$/, '')}',
    workspace: '${t.api.split('/').pop()}',
    token: TOKEN,
    // What the agent's conversations are filed under. With a token, history stays in this
    // browser; history: 'server' also needs the token's flow_conversations:write scope.
    flowPath: '${t.agentPath}.chat',
    // Runs the agent for each turn; the conversation id is its memory.
    run: async (args, { conversationId, signal }) => {
      const res = await fetch(\`\${API}/jobs/run/agent/${t.agentPath}?memory_id=\${conversationId}\`, {
        method: 'POST',
        headers: { Authorization: \`Bearer \${TOKEN}\`, 'Content-Type': 'application/json' },
        body: JSON.stringify(args),
        signal
      })
      if (!res.ok) throw new Error(await res.text())
      return (await res.text()).trim()
    }
  })
  // chat.messages (streamed as they are written), chat.status, chat.sendMessage(text), chat.stop(),
  // and the conversations: chat.conversations, chat.selectConversation(id), chat.newConversation()
}`
}

/** A brief for a coding assistant asked to integrate the agent. The token stays a placeholder. */
export function integrationPrompt(t: AgentApiTarget): string {
	const conversation = t.chat
		? `
## Conversations

The agent keeps the conversation. Pass \`memory_id\` as a query parameter on every run: any
string names a conversation, a UUID is best. Reuse it for follow-ups; a new one starts over.
Without it each message is answered on its own.

Windmill also keeps each conversation, as the token's owner's. One token shared by all your
users makes every conversation theirs, so keep your users' conversation ids in your own app.
Reading them back needs a token with the \`flow_conversations:read\` scope (\`:write\` to delete):
- \`GET ${t.api}/flow_conversations/list?flow_path=${t.agentPath}.chat\` (\`page\`, \`per_page\`).
  A conversation's \`id\` is the \`memory_id\` when that was a UUID.
- \`GET ${t.api}/flow_conversations/{id}/messages\`: its latest messages, oldest first.
- \`DELETE ${t.api}/flow_conversations/delete/{id}\`.

For a chat UI in React, the \`windmill-chat\` library (https://github.com/windmill-labs/windmill/tree/main/chat-sdk)
does all of this: \`useWindmillChat\` from \`windmill-chat/react\`, or the Vercel AI SDK transport
(\`windmill-chat/ai-sdk\`) and assistant-ui runtime (\`windmill-chat/assistant-ui\`). Point it at the
agent with \`flowPath: '${t.agentPath}.chat'\` and a \`run(args, { conversationId, signal })\` option
that POSTs \`args\` to the run endpoint with \`memory_id=conversationId\` and returns the job id.
`
		: ''
	return `Integrate the Windmill AI agent \`${t.agentPath}\` into this project.

## Authentication

Every request sends \`Authorization: Bearer <WINDMILL_TOKEN>\`. Read the token from configuration,
never from code shipped to a browser: the agent runs as the token's owner, with their permissions.

## Sending a message

\`POST ${runUrl(t)}\` with the JSON body \`{"user_message": "..."}\` starts a run and returns its
job id as plain text. An error is a non-2xx status with the reason as text.
${conversation}
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
