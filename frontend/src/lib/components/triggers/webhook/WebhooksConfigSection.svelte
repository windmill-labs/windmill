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
	import { ArrowDownRight, ArrowUpRight, Clipboard } from 'lucide-svelte'
	import { Highlight } from 'svelte-highlight'
	import { typescript } from 'svelte-highlight/languages'
	import ClipboardPanel from '../../details/ClipboardPanel.svelte'
	import { copyToClipboard, isObject } from '$lib/utils'
	// import { page } from '$app/stores'
	import { base } from '$lib/base'
	import TriggerTokens from '../TriggerTokens.svelte'
	import { workspaceStore, userStore } from '$lib/stores'
	import UserSettings from '../../UserSettings.svelte'
	import { generateRandomString } from '$lib/utils'

	export let isFlow: boolean = false
	export let path: string = ''
	export let hash: string | undefined = undefined
	export let token: string = ''
	export let runnableArgs: any
	export let triggerTokens: TriggerTokens | undefined = undefined
	export let scopes: string[] = []

	let webhooks: {
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
	let selectedTab: string = 'rest'
	let userSettings: UserSettings
	let url: string = ''

	$: webhooks = isFlow ? computeFlowWebhooks(path) : computeScriptWebhooks(hash, path)

	function computeScriptWebhooks(hash: string | undefined, path: string) {
		let webhookBase = `${location.origin}${base}/api/w/${$workspaceStore}/jobs`
		return {
			async: {
				hash: `${webhookBase}/run/h/${hash}`,
				path: `${webhookBase}/run/p/${path}`
			},
			sync: {
				hash: `${webhookBase}/run_wait_result/h/${hash}`,
				path: `${webhookBase}/run_wait_result/p/${path}`,
				get_path: `${webhookBase}/run_wait_result/p/${path}`
			}
		}
	}

	function computeFlowWebhooks(path: string) {
		let webhooksBase = `${location.origin}${base}/api/w/${$workspaceStore}/jobs`

		let urlAsync = `${webhooksBase}/run/f/${path}`
		let urlSync = `${webhooksBase}/run_wait_result/f/${path}`
		return {
			async: {
				path: urlAsync
			},
			sync: {
				path: urlSync,
				get_path: urlSync
			}
		}
	}

	let webhookType: 'async' | 'sync' = DEFAULT_WEBHOOK_TYPE
	let requestType: 'hash' | 'path' | 'get_path' = isFlow ? 'path' : 'path'
	let tokenType: 'query' | 'headers' = 'headers'

	$: if (webhookType === 'async' && requestType === 'get_path') {
		requestType = hash ? 'hash' : 'path'
	}

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

	$: cleanedRunnableArgs =
		isObject(runnableArgs) && 'wm_trigger' in runnableArgs
			? Object.fromEntries(Object.entries(runnableArgs).filter(([key]) => key !== 'wm_trigger'))
			: runnableArgs

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
			: `const body = JSON.stringify(${JSON.stringify(
					cleanedRunnableArgs ?? {},
					null,
					2
				).replaceAll('\n', '\n\t')});`
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
  const body = JSON.stringify(${JSON.stringify(cleanedRunnableArgs ?? {}, null, 2).replaceAll(
		'\n',
		'\n\t'
	)});
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

	function curlCode() {
		return `TOKEN='${token}'
${requestType !== 'get_path' ? `BODY='${JSON.stringify(cleanedRunnableArgs ?? {})}'` : ''}
URL='${url}'
${webhookType === 'sync' ? 'RESULT' : 'UUID'}=$(curl -s ${
			requestType != 'get_path' ? "-H 'Content-Type: application/json'" : ''
		} ${tokenType === 'headers' ? `-H "Authorization: Bearer $TOKEN"` : ''} -X ${
			requestType === 'get_path' ? 'GET' : 'POST'
		} ${requestType !== 'get_path' ? `-d "$BODY" ` : ''}$URL)

${
	webhookType === 'sync'
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

	$: url =
		webhooks[webhookType][requestType] +
		(tokenType === 'query'
			? `?token=${token}${
					requestType === 'get_path'
						? `&payload=${encodeURIComponent(btoa(JSON.stringify(cleanedRunnableArgs ?? {})))}`
						: ''
				}`
			: `${
					requestType === 'get_path'
						? `?payload=${encodeURIComponent(btoa(JSON.stringify(cleanedRunnableArgs ?? {})))}`
						: ''
				}`)
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
				<Button size="xs" color="light" variant="border" on:click={userSettings.openDrawer}>
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
			<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={webhookType} let:item>
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
			</ToggleButtonGroup>
		</div>
		<div class="flex flex-row justify-between">
			<div class="text-sm font-normal text-secondary flex flex-row items-center">Call method</div>
			<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={requestType} let:item>
				<ToggleButton
					label="POST by path"
					value="path"
					icon={ArrowUpRight}
					{item}
					selectedColor="#fb923c"
				/>
				{#if !isFlow}
					<ToggleButton
						label="POST by hash"
						value="hash"
						icon={ArrowUpRight}
						selectedColor="#fb923c"
						disabled={!hash}
						{item}
					/>
				{/if}

				<ToggleButton
					label="GET by path"
					value="get_path"
					icon={ArrowDownRight}
					disabled={webhookType !== 'sync'}
					{item}
					selectedColor="#14b8a6"
				/>
			</ToggleButtonGroup>
		</div>
		<div class="flex flex-row justify-between">
			<div class="text-sm font-normal text-secondary flex flex-row items-center"
				>Token configuration</div
			>
			<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={tokenType} let:item>
				<ToggleButton label="Token in Headers" value="headers" {item} />
				<ToggleButton label="Token in Query" value="query" {item} />
			</ToggleButtonGroup>
		</div>
	</div>
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div>
		<Tabs bind:selected={selectedTab}>
			<Tab value="rest" size="xs">REST</Tab>
			{#if SCRIPT_VIEW_SHOW_EXAMPLE_CURL}
				<Tab value="curl" size="xs">Curl</Tab>
			{/if}
			<Tab value="fetch" size="xs">Fetch</Tab>

			<svelte:fragment slot="content">
				{#key token}
					<TabContent value="rest" class="flex flex-col flex-1 h-full ">
						<div class="flex flex-col gap-2">
							<Label label="Url">
								<ClipboardPanel content={url} />
							</Label>

							{#if requestType !== 'get_path'}
								<Label label="Body">
									<ClipboardPanel content={JSON.stringify(cleanedRunnableArgs ?? {}, null, 2)} />
								</Label>
							{/if}
							{#key requestType}
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
				{/key}
			</svelte:fragment>
		</Tabs>
	</div>
	<TriggerTokens bind:this={triggerTokens} {isFlow} {path} labelPrefix="webhook" />
</div>
