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
	export let token: string
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
		if (webhookType === 'sync') {
			return `
export async function main() {
  const jobTriggerResponse = await triggerJob();
  const data = await jobTriggerResponse.json();
  return data;
}

async function triggerJob() {
  ${
		requestType === 'get_path'
			? '// Payload is a base64 encoded string of the arguments'
			: `const body = JSON.stringify(${JSON.stringify(args, null, 2).replaceAll('\n', '\n\t')});`
	}
  const endpoint = \`${url}\`;

  return await fetch(endpoint, {
    method: '${requestType === 'get_path' ? 'GET' : 'POST'}',
    headers: ${JSON.stringify(headers(), null, 2).replaceAll('\n', '\n\t\t')}${
				requestType === 'get_path' ? '' : `,\n\t\tbody`
			}
  });
}`
		}

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
  const body = JSON.stringify(${JSON.stringify(args, null, 2).replaceAll('\n', '\n\t')});
  const endpoint = \`${url}\`;

  return await fetch(endpoint, {
    method: '${requestType === 'get_path' ? 'GET' : 'POST'}',
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
				$page.url.origin
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

	function curlCode() {
		return `TOKEN='${token}'
${requestType !== 'get_path' ? `BODY='${JSON.stringify(args)}'` : ''}
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

<UserSettings bind:this={userSettings} {scopes} />

<div class="p-2 flex flex-col w-full gap-4">
	{#if SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON}
		<div class="flex flex-row justify-between my-2 gap-2">
			<input
				bind:value={token}
				placeholder="paste your token here once created to alter examples below"
				class="!text-xs"
			/>
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
					tooltip="The returning value is the uuid of the job assigned to execute the job."
				/>
				<ToggleButton
					label="Sync"
					value="sync"
					tooltip="Triggers the execution, wait for the job to complete and return it as a response."
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
	{#key token}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
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
	{/key}
</div>
