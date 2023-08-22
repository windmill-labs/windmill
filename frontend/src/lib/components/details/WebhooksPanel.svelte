<script lang="ts">
	import { page } from '$app/stores'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { workspaceStore } from '$lib/stores'
	import bash from 'svelte-highlight/languages/bash'
	import { Tabs, Tab, TabContent, Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import {
		DEFAULT_WEBHOOK_TYPE,
		SCRIPT_VIEW_SHOW_EXAMPLE_CURL,
		SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON
	} from '$lib/consts'
	import { ArrowDownRight, ArrowUpRight, Clipboard } from 'lucide-svelte'
	import UserSettings from '../UserSettings.svelte'
	import { Highlight } from 'svelte-highlight'
	import { typescript } from 'svelte-highlight/languages'
	import ClipboardPanel from './ClipboardPanel.svelte'
	import { copyToClipboard } from '$lib/utils'

	let userSettings: UserSettings

	export let webhooks: {
		async: {
			hash?: string
			path: string
		}
		sync: {
			hash?: string
			path: string
			get_path?: string
		}
	}
	export let args: any
	export let scopes: string[] = []
	export let isFlow: boolean = false

	let webhookType: 'async' | 'sync' = DEFAULT_WEBHOOK_TYPE
	let requestType: 'hash' | 'path' | 'get_path' = isFlow ? 'path' : 'path'
	let tokenType: 'query' | 'headers' = 'headers'

	$: if (webhookType === 'async' && requestType === 'get_path') {
		// Request type is not supported for async webhooks
		requestType = 'hash'
	}

	let token = 'YOUR_TOKEN'

	$: url =
		webhooks[webhookType][requestType] +
		(tokenType === 'query'
			? `?token=${token}${
					requestType === 'get_path'
						? `&payload=${encodeURIComponent(btoa(JSON.stringify(args)))}`
						: ''
			  }`
			: `${
					requestType === 'get_path'
						? `?payload=${encodeURIComponent(btoa(JSON.stringify(args)))}`
						: ''
			  }`)

	function headers() {
		const headers = {}
		if (requestType != 'get_path') {
			headers['Content-Type'] = 'application/json'
		}

		if (tokenType === 'headers') {
			headers['Authorization'] = `Bearer ${token}`
		}
		return headers
	}

	function fetchCode() {
    let fetchMain = `
const url = \`${url}\`;
const response = await fetch(url, {
	method: '${requestType === 'get_path' ? 'GET' : 'POST'}',
	headers: ${JSON.stringify(headers(), null, 2).replaceAll('\n', '\n\t')},
	${requestType !== 'get_path' ? `body` : ''}
});

const data = await response.${webhookType === 'sync' ? 'json' : 'text'}();
    `;

    let fetchData = webhookType === 'sync'
        ? `console.log(data);`
        : `
const UUID = data;
let checkCompletion = setInterval(async () => {
	try {
		let completionResponse = await fetch(\`${$page.url.origin}/api/w/${$workspaceStore}/jobs_u/completed/get_result_maybe/\$\{UUID\}\`, {
			method: 'GET',
			headers: {
					'Authorization': 'Bearer ${token}'
			}
		});
		let completionData = await completionResponse.json();
		if (completionData.completed) {
			console.log(completionData.result);
			clearInterval(checkCompletion);
		}
	} catch (error) {
		console.error("Error checking completion:", error);
	}
}, 1000);`;

    let fetchCodeString = `${requestType !== 'get_path'? 'const body = JSON.stringify(' + JSON.stringify(args, null, 2) + ');': '' }
${fetchMain}
${fetchData}`;

    return fetchCodeString;
}


	function curlCode() {
		return `${
			requestType !== 'get_path'
				? `TOKEN='${token}'
BODY='${JSON.stringify(args)}'`
				: ''
		}
URL='${url}'
${webhookType === 'sync' ? 'RESULT' : 'UUID'}=$(curl -s ${
			requestType != 'get_path' ? "-H 'Content-Type: application/json'" : ''
		} ${tokenType === 'headers' ? `-H "Authorization: Bearer $TOKEN"` : ''} -X ${
			requestType === 'get_path' ? 'GET' : 'POST'
		} ${requestType !== 'get_path' ? `-d "$BODY" ` : ''}$URL)

${
	webhookType === 'sync'
		? 'echo $RESULT | jq'
		: `
URL="${$page.url.origin}/api/w/${$workspaceStore}/jobs_u/completed/get_result_maybe/$UUID"
while true; do
	RESPONSE=$(curl -s -H "Authorization: Bearer $TOKEN" $URL)
	COMPLETED=$(echo $RESPONSE | jq .completed)
	if [ "$COMPLETED" = "true" ]; then
		echo $RESPONSE | jq .result
		break
	else
		sleep 1
	fi
done`
}`
	}
</script>

<UserSettings bind:this={userSettings} {scopes} />

<div class="p-2 flex flex-col w-full gap-4">
	{#if SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON}
		<div class="flex flex-row justify-between my-2 gap-2">
			<input bind:value={token} placeholder="YOUR_TOKEN" class="!text-xs" />
			<Button size="xs" color="light" variant="border" on:click={userSettings.openDrawer}>
				Create a Webhook-specific Token
				<Tooltip light>
					The token will have a scope such that it can only be used to trigger this script. It is
					safe to share as it cannot be used to impersonate you.
				</Tooltip>
			</Button>
		</div>
	{/if}

	<div class="flex flex-col gap-2">
		<div class="flex flex-row justify-between">
			<div class="text-xs font-semibold flex flex-row items-center">Request type</div>
			<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={webhookType}>
				<ToggleButton
					label="Async"
					value="async"
					tooltip="Jobs can be triggered in asynchronous mode, meaning that the webhook is triggered, and the returning value is the uuid of the job assigned to execute the underlying code."
				/>
				<ToggleButton
					label="Sync"
					value="sync"
					tooltip="The second type of autogenerated endpoint is the synchronous webhook. This webhook triggers the execution, automatically extracts the underlying code's return value and returns it as the response."
				/>
			</ToggleButtonGroup>
		</div>
		<div class="flex flex-row justify-between">
			<div class="text-xs font-semibold flex flex-row items-center">Call method</div>
			<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={requestType}>
				<ToggleButton
					label="POST by path"
					value="path"
					icon={ArrowUpRight}
					selectedColor="#fb923c"
				/>
				{#if !isFlow}
					<ToggleButton
						label="POST by hash"
						value="hash"
						icon={ArrowUpRight}
						selectedColor="#fb923c"
					/>
				{/if}

				<ToggleButton
					label="GET by path"
					value="get_path"
					icon={ArrowDownRight}
					disabled={webhookType !== 'sync'}
					selectedColor="#14b8a6"
				/>
			</ToggleButtonGroup>
		</div>
		<div class="flex flex-row justify-between">
			<div class="text-xs font-semibold flex flex-row items-center">Token configuration</div>
			<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={tokenType}>
				<ToggleButton label="Token in Headers" value="headers" />
				<ToggleButton label="Token in Query" value="query" />
			</ToggleButtonGroup>
		</div>
	</div>

	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<Tabs selected="rest">
		<Tab value="rest" size="xs">REST</Tab>
		{#if SCRIPT_VIEW_SHOW_EXAMPLE_CURL}
			<Tab value="curl" size="xs">Curl</Tab>
		{/if}
		<Tab value="fetch" size="xs">Fetch</Tab>

		<svelte:fragment slot="content">
			<TabContent value="rest" class="flex flex-col flex-1 h-full ">
				<div class="flex flex-col gap-2">
					<ClipboardPanel title="Url" content={url} />

					{#if requestType !== 'get_path'}
						<ClipboardPanel title="Body" content={JSON.stringify(args, null, 2)} />
					{/if}
					{#key requestType}
						{#key tokenType}
							<ClipboardPanel title="Headers" content={JSON.stringify(headers(), null, 2)} />
						{/key}
					{/key}
				</div>
			</TabContent>
			<TabContent value="curl" class="flex flex-col flex-1 h-full">
				<div class="relative">
					{#key args}
						{#key requestType}
							{#key webhookType}
								{#key tokenType}
									<div
										class="flex flex-row flex-1 h-full border p-2 rounded-md overflow-auto relative"
										on:click={(e) => {
											e.preventDefault()
											copyToClipboard(curlCode())
										}}
									>
										<Highlight language={bash} code={curlCode()} class="" />
										<Clipboard size={14} class="w-8 top-2 right-2 absolute" />
									</div>
								{/key}
							{/key}
						{/key}
					{/key}
				</div>
			</TabContent>
			<TabContent value="fetch">
				{#key args}
					{#key requestType}
						{#key webhookType}
							{#key tokenType}
							{#key token}
								<div
									class="flex flex-row flex-1 h-full border p-2 rounded-md overflow-auto relative"
									on:click={(e) => {
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
		</svelte:fragment>
	</Tabs>
</div>
