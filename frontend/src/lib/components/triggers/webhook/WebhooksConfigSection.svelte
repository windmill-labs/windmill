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
	import { ArrowDownRight, ArrowUpRight, Clipboard, RssIcon } from 'lucide-svelte'
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

	interface Props {
		isFlow?: boolean
		path?: string
		hash?: string | undefined
		token?: string
		runnableArgs: any
		triggerTokens?: TriggerTokens | undefined
		scopes?: string[]
	}

	let {
		isFlow = false,
		path = '',
		hash = undefined,
		token = $bindable(''),
		runnableArgs,
		triggerTokens = $bindable(undefined),
		scopes = []
	}: Props = $props()

	let webhooks: {
		async: {
			get: {}
			post: {
				hash?: string
				path: string
			}
			sse: {}
		}
		sync: {
			get: {
				path: string
			}
			post: {
				hash?: string
				path: string
			}
			sse: {
				hash?: string
				path: string
			}
		}
	} = $derived(isFlow ? computeFlowWebhooks(path) : computeScriptWebhooks(hash, path))
	let selectedTab: string = $state('rest')
	let userSettings: UserSettings | undefined = $state()
	let requestType = $state(DEFAULT_WEBHOOK_TYPE) as 'async' | 'sync'
	let callMethod = $state('post') as 'get' | 'post' | 'sse'
	let runnableId = $state('path') as 'hash' | 'path'
	let tokenType = $state('headers') as 'query' | 'headers'

	$effect(() => {
		if (requestType === 'async' && (callMethod === 'get' || callMethod === 'sse')) {
			callMethod = 'post'
		}
	})

	$effect(() => {
		if (callMethod === 'sse' && tokenType === 'headers') {
			tokenType = 'query'
		}
	})

	let cleanedRunnableArgs = $derived.by(() => {
		readFieldsRecursively(runnableArgs)
		return isObject(runnableArgs) && 'wm_trigger' in runnableArgs
			? Object.fromEntries(Object.entries(runnableArgs).filter(([key]) => key !== 'wm_trigger'))
			: runnableArgs
	})
	let url: string = $derived(
		webhooks[requestType][callMethod][runnableId] +
			(tokenType === 'query'
				? `?token=${token}${
						callMethod === 'get' || callMethod === 'sse'
							? `&payload=${encodeURIComponent(btoa(JSON.stringify(cleanedRunnableArgs ?? {})))}`
							: ''
					}`
				: `${
						callMethod === 'get'
							? `?payload=${encodeURIComponent(btoa(JSON.stringify(cleanedRunnableArgs ?? {})))}`
							: ''
					}`)
	)

	function computeScriptWebhooks(hash: string | undefined, path: string) {
		let webhookBase = `${location.origin}${base}/api/w/${$workspaceStore}/jobs`
		return {
			async: {
				get: {},
				post: {
					hash: `${webhookBase}/run/h/${hash}`,
					path: `${webhookBase}/run/p/${path}`
				},
				sse: {}
			},
			sync: {
				get: {
					path: `${webhookBase}/run_wait_result/p/${path}`
				},
				post: {
					hash: `${webhookBase}/run_wait_result/h/${hash}`,
					path: `${webhookBase}/run_wait_result/p/${path}`
				},
				sse: {
					hash: `${webhookBase}/run_and_stream/h/${hash}`,
					path: `${webhookBase}/run_and_stream/p/${path}`
				}
			}
		}
	}

	function computeFlowWebhooks(path: string) {
		let webhooksBase = `${location.origin}${base}/api/w/${$workspaceStore}/jobs`

		let urlAsync = `${webhooksBase}/run/f/${path}`
		let urlSync = `${webhooksBase}/run_wait_result/f/${path}`
		let urlStream = `${webhooksBase}/run_and_stream/f/${path}`
		return {
			async: {
				get: {},
				post: {
					path: urlAsync
				},
				sse: {}
			},
			sync: {
				get: {
					path: urlSync
				},
				post: {
					path: urlSync
				},
				sse: {
					path: urlStream
				}
			}
		}
	}

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
		if (callMethod === 'sse') {
			return `
import { EventSource } from "eventsource";

export async function main() {
	const endpoint = \`${url}\`;

	return new Promise((resolve, reject) => {
		const eventSource = new EventSource(endpoint);

		eventSource.onmessage = (event) => {
			const data = JSON.parse(event.data);
			console.log(data);
			if (data.completed) {
				eventSource.close();
				resolve();
			}
		};

		eventSource.onerror = (error) => {
			console.error('EventSource error:', error);
			eventSource.close();
			reject(error);
		};
	});
}`
		}
		if (requestType === 'sync') {
			return `
export async function main() {
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
			let mainFunction = `
export async function main() {
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
${requestType === 'sync' ? 'RESULT' : 'UUID'}=$(curl -s ${
			callMethod != 'get' ? "-H 'Content-Type: application/json'" : ''
		} ${tokenType === 'headers' ? `-H "Authorization: Bearer $TOKEN"` : ''} -X ${
			callMethod === 'get' ? 'GET' : 'POST'
		} ${callMethod !== 'get' ? `-d "$BODY" ` : ''}$URL)

${
	requestType === 'sync'
		? 'echo -E $RESULT | jq'
		: `
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
			<div class="flex flex-row justify-between gap-2">
				<input
					bind:value={token}
					placeholder="paste your token here once created to alter examples below"
					class="!text-xs !font-normal"
				/>
				<Button
					size="xs"
					color="light"
					variant="border"
					on:click={() => userSettings?.openDrawer()}
				>
					Create a Webhook-specific Token
					<Tooltip light>
						The token will have a scope such that it can only be used to trigger this script. It is
						safe to share as it cannot be used to impersonate you.
					</Tooltip>
				</Button>
			</div>
		</Label>
	{/if}

	<div class="flex flex-col gap-2">
		<div class="flex flex-row justify-between">
			<div class="text-sm font-normal text-secondary flex flex-row items-center">Request type</div>
			<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={requestType}>
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
				{/snippet}
			</ToggleButtonGroup>
		</div>
		<div class="flex flex-row justify-between">
			<div class="text-sm font-normal text-secondary flex flex-row items-center">Call method</div>
			<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={callMethod}>
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
						disabled={requestType !== 'sync'}
					/>
					<ToggleButton
						label="SSE"
						value="sse"
						icon={RssIcon}
						selectedColor="#3B82F6"
						disabled={requestType !== 'sync'}
						tooltip={'Returns an SSE stream. ' +
							(isFlow
								? 'Only useful if the last step of the flow returns a stream.'
								: 'Only useful if the script returns a stream.')}
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
		</div>
		{#if !isFlow}
			<div class="flex flex-row justify-between">
				<div class="text-sm font-normal text-secondary flex flex-row items-center">
					Reference type
				</div>
				<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={runnableId}>
					{#snippet children({ item })}
						<ToggleButton label="Path" value="path" {item} />
						<ToggleButton label="Hash" value="hash" disabled={!hash} {item} />
					{/snippet}
				</ToggleButtonGroup>
			</div>
		{/if}
		<div class="flex flex-row justify-between">
			<div class="text-sm font-normal text-secondary flex flex-row items-center"
				>Token configuration</div
			>
			<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={tokenType}>
				{#snippet children({ item })}
					<ToggleButton
						label="Token in Headers"
						value="headers"
						{item}
						disabled={callMethod === 'sse'}
					/>
					<ToggleButton label="Token in Query" value="query" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</div>
	</div>

	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div>
		<Tabs bind:selected={selectedTab}>
			<Tab value="rest" size="xs">REST</Tab>
			{#if SCRIPT_VIEW_SHOW_EXAMPLE_CURL && callMethod !== 'sse'}
				<Tab value="curl" size="xs">Curl</Tab>
			{/if}
			<Tab value="fetch" size="xs">
				{callMethod === 'sse' ? 'Event Source' : 'Fetch'}
			</Tab>

			{#snippet content()}
				{#key token}
					<TabContent value="rest" class="flex flex-col flex-1 h-full ">
						<div class="flex flex-col gap-2">
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
					<TabContent value="curl" class="flex flex-col flex-1 h-full">
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
												<Clipboard size={14} class="w-8 top-2 right-2 absolute" />
											</div>
										{/key}
									{/key}
								{/key}
							{/key}
						</div>
					</TabContent>
					<TabContent value="fetch">
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
												<Clipboard size={14} class="w-8 top-2 right-2 absolute" />
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
