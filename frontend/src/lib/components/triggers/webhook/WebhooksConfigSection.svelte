<script lang="ts">
	import Label from '../../Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import {
		DEFAULT_WEBHOOK_TYPE,
		SCRIPT_VIEW_SHOW_EXAMPLE_CURL,
		SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON
	} from '$lib/consts'
	import bash from 'svelte-highlight/languages/bash'
	import { Tabs, Tab, TabContent, Button } from '$lib/components/common'
	import { ArrowDownRight, ArrowUpRight, Copy } from 'lucide-svelte'
	import { Highlight } from 'svelte-highlight'
	import { typescript } from 'svelte-highlight/languages'
	import ClipboardPanel from '../../details/ClipboardPanel.svelte'
	import { copyToClipboard, isObject, readFieldsRecursively } from '$lib/utils'
	// import { page } from '$app/stores'
	import { base } from '$lib/base'
	import TriggerTokens from '../TriggerTokens.svelte'
	import { workspaceStore, userStore } from '$lib/stores'
	import UserSettings from '../../UserSettings.svelte'
	import { generateRandomString } from '$lib/utils'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		isFlow?: boolean
		path?: string
		runnableVersion?: string | undefined
		token?: string
		runnableArgs: any
		triggerTokens?: TriggerTokens | undefined
		scopes?: string[]
	}

	let {
		isFlow = false,
		path = '',
		runnableVersion = undefined,
		token = $bindable(''),
		runnableArgs,
		triggerTokens = $bindable(undefined),
		scopes = []
	}: Props = $props()

	const WEBHOOK_BASE_URL = `${location.origin}${base}/api/w/${$workspaceStore}/jobs`

	let baseWebhookUrl = $derived.by(() => {
		let webhookUrlPath: string

		if (isFlow) {
			webhookUrlPath = runnableId == 'path' ? `f/${path}` : `fv/${runnableVersion}`
		} else {
			webhookUrlPath = runnableId == 'path' ? `p/${path}` : `h/${runnableVersion}`
		}

		if (requestType == 'async') {
			return `${WEBHOOK_BASE_URL}/run/${webhookUrlPath}`
		} else if (requestType == 'sync') {
			return `${WEBHOOK_BASE_URL}/run_wait_result/${webhookUrlPath}`
		} else {
			return `${WEBHOOK_BASE_URL}/run_and_stream/${webhookUrlPath}`
		}
	})

	let selectedTab: string = $state('rest')
	let userSettings: UserSettings | undefined = $state()
	let requestType = $state(DEFAULT_WEBHOOK_TYPE) as 'async' | 'sync' | 'sync_sse'
	let callMethod = $state('post') as 'get' | 'post'
	let runnableId = $state('path') as 'runnableVersion' | 'path'
	let tokenType = $state('headers') as 'query' | 'headers'

	$effect(() => {
		if (requestType === 'async' && callMethod === 'get') {
			callMethod = 'post'
		}
	})

	let cleanedRunnableArgs = $derived.by(() => {
		readFieldsRecursively(runnableArgs)
		return isObject(runnableArgs) && 'wm_trigger' in runnableArgs
			? Object.fromEntries(Object.entries(runnableArgs).filter(([key]) => key !== 'wm_trigger'))
			: runnableArgs
	})
	let url: string = $derived(
		baseWebhookUrl +
			(tokenType === 'query'
				? `?token=${token}${
						callMethod === 'get' || requestType === 'sync_sse'
							? `&payload=${encodeURIComponent(btoa(JSON.stringify(cleanedRunnableArgs ?? {})))}`
							: ''
					}`
				: `${
						callMethod === 'get'
							? `?payload=${encodeURIComponent(btoa(JSON.stringify(cleanedRunnableArgs ?? {})))}`
							: ''
					}`)
	)

	function headers() {
		const headers = {}
		if (callMethod === 'post') {
			headers['Content-Type'] = 'application/json'
		}

		if (tokenType === 'headers') {
			headers['Authorization'] = `Bearer ${token}`
		}
		return headers
	}

	function fetchCode() {
		if (requestType === 'sync_sse') {
			return `import { EventSource } from "eventsource";

export async function main() {
  const response = await fetch(\`${url}\`, {
    method: '${callMethod === 'get' ? 'GET' : 'POST'}',
    headers: ${JSON.stringify(headers(), null, 2).replaceAll('\n', '\n    ')},
    body: ${callMethod === 'get' ? 'undefined' : `JSON.stringify(${JSON.stringify(cleanedRunnableArgs ?? {}, null, 2).replaceAll('\n', '\n    ')})`}
  });
  
  if (!response.ok) {
	const text = await response.text()
	throw new Error(\`\${response.status} \${text}\`)
  }

  if (!response.body) {
    throw new Error("Response body is empty");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      const { done, value } = await reader.read();

      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split("\\n");

      // Keep the last incomplete line in buffer
      buffer = lines.pop() || "";

      for (const line of lines) {
        if (line.startsWith("data: ")) {
          const jsonData = line.slice(6); // Remove 'data: ' prefix
          const data = JSON.parse(jsonData);
          console.log(data);

          if (data.completed) {
            reader.cancel();
            return;
          }
        }
      }
    }
  } catch (error) {
    console.error("Stream error:", error);
    throw error;
  }
}`
		}
		if (requestType === 'sync') {
			return `export async function main() {
  const jobTriggerResponse = await triggerJob();
  const data = await jobTriggerResponse.json();
  return data;
}

async function triggerJob() {
  ${
		callMethod === 'get'
			? '// Payload is a base64 encoded string of the arguments'
			: `const body = JSON.stringify(${JSON.stringify(
					cleanedRunnableArgs ?? {},
					null,
					2
				).replaceAll('\n', '\n\t')});`
	}
  const endpoint = \`${url}\`;

  return await fetch(endpoint, {
    method: '${callMethod === 'get' ? 'GET' : 'POST'}',
    headers: ${JSON.stringify(headers(), null, 2).replaceAll('\n', '\n\t\t')}${
			callMethod === 'get' ? '' : `,\n\t\tbody`
		}
  });
}`
		} else {
			// Main function
			let mainFunction = `export async function main() {
  const jobTriggerResponse = await triggerJob();
  const UUID = await jobTriggerResponse.text();
  const jobCompletionData = await waitForJobCompletion(UUID);
  return jobCompletionData;
}`

			// triggerJob function
			let triggerJobFunction = `
async function triggerJob() {
  const body = JSON.stringify(${JSON.stringify(cleanedRunnableArgs ?? {}, null, 2).replaceAll(
		'\n',
		'\n\t'
	)});
  const endpoint = \`${url}\`;

  return await fetch(endpoint, {
    method: '${callMethod === 'get' ? 'GET' : 'POST'}',
    headers: ${JSON.stringify(headers(), null, 2).replaceAll('\n', '\n\t\t')},
    body
  });
}`

			// waitForJobCompletion function
			let waitForJobCompletionFunction = `
function waitForJobCompletion(UUID) {
  return new Promise(async (resolve, reject) => {
    try {
      const endpoint = \`${
				location.origin
			}/api/w/${$workspaceStore}/jobs_u/completed/get_result_maybe/\${UUID}\`;
      const checkResponse = await fetch(endpoint, {
        method: 'GET',
        headers: ${JSON.stringify(headers(), null, 2).replaceAll('\n', '\n\t\t\t\t')}
      });

      const checkData = await checkResponse.json();

      if (checkData.completed) {
        resolve(checkData);
      } else {
        // If not completed, wait for a second then try again
        setTimeout(async () => {
          const result = await waitForJobCompletion(UUID);
          resolve(result);
        }, 1000);
      }
    } catch (error) {
      reject(error);
    }
  });
}`

			// Combine and return
			return `${mainFunction}\n\n${triggerJobFunction}\n\n${waitForJobCompletionFunction}`
		}
	}

	function curlCode() {
		return `TOKEN='${token}'
${callMethod !== 'get' ? `BODY='${JSON.stringify(cleanedRunnableArgs ?? {})}'` : ''}
URL='${url}'
${requestType === 'sync' ? 'RESULT=$(' : requestType === 'async' ? 'UUID=$(' : ''}curl -s ${
			callMethod != 'get' ? "-H 'Content-Type: application/json'" : ''
		} ${tokenType === 'headers' ? `-H "Authorization: Bearer $TOKEN"` : ''} -X ${
			callMethod === 'get' ? 'GET' : 'POST'
		} ${callMethod !== 'get' ? `-d "$BODY" ` : ''}$URL${requestType === 'sync' || requestType === 'async' ? ')' : ''}

${
	requestType === 'sync'
		? 'echo -E $RESULT | jq'
		: requestType === 'async'
			? `
URL="${location.origin}/api/w/${$workspaceStore}/jobs_u/completed/get_result_maybe/$UUID"
while true; do
  curl -s -H "Authorization: Bearer $TOKEN" $URL -o res.json
  COMPLETED=$(cat res.json | jq .completed)
  if [ "$COMPLETED" = "true" ]; then
    cat res.json | jq .result
    break
  else
    sleep 1
  fi
done`
			: ''
}`
	}
</script>

<UserSettings
	bind:this={userSettings}
	on:tokenCreated={(e) => {
		token = e.detail
		triggerTokens?.listTokens()
	}}
	newTokenWorkspace={$workspaceStore}
	newTokenLabel={`webhook-${$userStore?.username ?? 'superadmin'}-${generateRandomString(4)}`}
	{scopes}
/>

<div class="flex flex-col gap-8">
	{#if SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON}
		<Label label="Token">
			<div class="flex flex-row justify-between gap-2 whitespace-nowrap">
				<TextInput
					bind:value={token}
					inputProps={{ placeholder: 'Paste your token here once created to alter examples below' }}
					class="!text-xs !font-normal"
				/>
				<Button size="xs" variant="default" on:click={() => userSettings?.openDrawer()}>
					Create a Webhook-specific Token
					<Tooltip light>
						The token will have a scope such that it can only be used to trigger this script. It is
						safe to share as it cannot be used to impersonate you.
					</Tooltip>
				</Button>
			</div>
		</Label>
	{/if}

	<div class="flex flex-col gap-6">
		<Label label="Request type">
			<ToggleButtonGroup bind:selected={requestType}>
				{#snippet children({ item })}
					<ToggleButton
						label="Async"
						value="async"
						tooltip="The returning value is the uuid of the job assigned to execute the job."
						{item}
					/>
					<ToggleButton
						label="Sync"
						value="sync"
						tooltip="Triggers the execution, wait for the job to complete and return it as a response."
						{item}
					/>
					<ToggleButton
						label="Sync SSE"
						value="sync_sse"
						tooltip={'Triggers the execution and returns an SSE stream. ' +
							(isFlow
								? 'Only useful if the last step of the flow returns a stream.'
								: 'Only useful if the script returns a stream.')}
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
		</Label>
		<Label label="Call method">
			<ToggleButtonGroup bind:selected={callMethod}>
				{#snippet children({ item })}
					<ToggleButton
						label="POST"
						icon={ArrowDownRight}
						selectedColor="#14b8a6"
						value="post"
						{item}
					/>
					<ToggleButton
						label="GET"
						icon={ArrowUpRight}
						selectedColor="#fb923c"
						value="get"
						{item}
						disabled={requestType !== 'sync' && requestType !== 'sync_sse'}
					/>
				{/snippet}
			</ToggleButtonGroup>
		</Label>
		<Label label="Reference type">
			<ToggleButtonGroup bind:selected={runnableId}>
				{#snippet children({ item })}
					<ToggleButton label="Path" value="path" {item} />
					<ToggleButton
						label={isFlow ? 'Flow version' : 'Hash'}
						value="runnableVersion"
						disabled={!runnableVersion}
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
		</Label>
		<Label label="Token configuration">
			<ToggleButtonGroup bind:selected={tokenType}>
				{#snippet children({ item })}
					<ToggleButton label="Token in Headers" value="headers" {item} />
					<ToggleButton label="Token in Query" value="query" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</Label>
	</div>

	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div>
		<Tabs bind:selected={selectedTab}>
			<Tab value="rest" label="REST" />
			{#if SCRIPT_VIEW_SHOW_EXAMPLE_CURL}
				<Tab value="curl" label="Curl" />
			{/if}
			<Tab value="fetch" label="Fetch" />

			{#snippet content()}
				{#key token}
					<TabContent value="rest" class="flex flex-col flex-1 h-full mt-2">
						<div class="flex flex-col gap-6">
							<Label label="Url">
								<ClipboardPanel content={url} />
							</Label>

							{#if callMethod !== 'get'}
								<Label label="Body">
									<ClipboardPanel content={JSON.stringify(cleanedRunnableArgs ?? {}, null, 2)} />
								</Label>
							{/if}
							{#key callMethod}
								{#key tokenType}
									<Label label="Headers">
										<ClipboardPanel content={JSON.stringify(headers(), null, 2)} />
									</Label>
								{/key}
							{/key}
						</div>
					</TabContent>
					<TabContent value="curl" class="flex flex-col flex-1 h-full mt-2">
						<div class="relative">
							{#key runnableArgs}
								{#key callMethod}
									{#key requestType}
										{#key tokenType}
											<div
												class="flex flex-row flex-1 h-full border p-2 rounded-md overflow-auto relative"
												onclick={(e) => {
													e.preventDefault()
													copyToClipboard(curlCode())
												}}
											>
												<Highlight language={bash} code={curlCode()} />
												<Copy size={14} class="w-8 top-2 right-2 absolute cursor-pointer" />
											</div>
										{/key}
									{/key}
								{/key}
							{/key}
						</div>
					</TabContent>
					<TabContent value="fetch" class="mt-2">
						{#key runnableArgs}
							{#key callMethod}
								{#key requestType}
									{#key tokenType}
										{#key token}
											<div
												class="flex flex-row flex-1 h-full border p-2 rounded-md overflow-auto relative"
												onclick={(e) => {
													e.preventDefault()
													copyToClipboard(fetchCode())
												}}
											>
												<Highlight language={typescript} code={fetchCode()} />
												<Copy size={14} class="w-8 top-2 right-2 absolute cursor-pointer" />
											</div>
										{/key}{/key}{/key}{/key}
						{/key}
					</TabContent>
				{/key}
			{/snippet}
		</Tabs>
	</div>
	<TriggerTokens bind:this={triggerTokens} {isFlow} {path} labelPrefix="webhook" />
</div>
