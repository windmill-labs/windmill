<script lang="ts">
	import { page } from '$app/stores'
	import CustomSelect from '../CustomSelect.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button, Popup } from '$lib/components/common'

	import { DEFAULT_WEBHOOK_TYPE } from '$lib/consts'
	import { Clipboard, PlusIcon } from 'lucide-svelte'

	import UserSettings from '../UserSettings.svelte'

	import { copyToClipboard, generateRandomString } from '$lib/utils'
	import HighlightTheme from '../HighlightTheme.svelte'
	import { base } from '$lib/base'
	let userSettings: UserSettings

	export let token: string
	export let args: any
	export let scopes: string[] = []
	export let isFlow: boolean = false
	export let hash: string | undefined = undefined
	export let path: string
	export let url: string = ''

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

	function computeScriptWebhooks(hash: string | undefined, path: string) {
		let webhookBase = `${$page.url.origin}${base}/api/w/${$workspaceStore}/jobs`
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
		let webhooksBase = `${$page.url.origin}${base}/api/w/${$workspaceStore}/jobs`

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

	$: webhooks = isFlow ? computeFlowWebhooks(path) : computeScriptWebhooks(hash, path)

	let webhookType: 'async' | 'sync' = DEFAULT_WEBHOOK_TYPE
	let requestType: 'hash' | 'path' | 'get_path' = isFlow ? 'path' : 'path'
	let tokenType: 'query' | 'headers' = 'headers'

	$: if (webhookType === 'async' && requestType === 'get_path') {
		// Request type is not supported for async webhooks
		requestType = 'hash'
	}

	const webhookTypeItems = [
		{ value: 'async', label: 'Async' },
		{ value: 'sync', label: 'Sync' }
	]

	const baseRequestTypeItems = [
		{ value: 'path', label: 'POST' },
		{ value: 'hash', label: 'POST by hash' },
		{ value: 'get_path', label: 'GET' }
	]

	$: requestTypeItems = baseRequestTypeItems.filter((item) => {
		if (isFlow && item.value === 'hash') return false
		if (webhookType !== 'sync' && item.value === 'get_path') return false
		return true
	})

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
		? 'echo -E $RESULT | jq'
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

	let isOpen = false

	let webhookTypeSelected

	$: webhookType = webhookTypeSelected ?? webhookType

	let webhookDisabled = true
	$: webhookDisabled = url === '' || token === ''
</script>

<HighlightTheme />

<UserSettings
	bind:this={userSettings}
	on:tokenCreated={(e) => {
		token = e.detail
	}}
	newTokenLabel={`webhook-${$userStore?.username ?? 'superadmin'}-${generateRandomString(4)}`}
	{scopes}
/>

<div class="flex flex-row text-xs grow w-full gap-2 items-center">
	<input
		bind:value={token}
		placeholder="paste your token here or create one"
		class="!text-xs grow"
	/>
	<Button
		title="Create a token with the correct scope to trigger this script"
		spacingSize="xs2"
		size="xs"
		color="light"
		variant="border"
		on:click={userSettings.openDrawer}
	>
		<PlusIcon size={12} />
	</Button>

	<CustomSelect
		bind:value={webhookTypeSelected}
		items={webhookTypeItems}
		placeholder="Type"
		selectClass="w-[60px] flex-none"
	/>

	<CustomSelect
		bind:value={requestType}
		items={requestTypeItems}
		placeholder="Method"
		selectClass="w-[70px] flex-none"
	/>

	<CustomSelect
		bind:value={tokenType}
		items={[
			{ value: 'headers', label: 'Token in Headers' },
			{ value: 'query', label: 'Token in Query' }
		]}
		placeholder="Token Location"
		selectClass="w-[120px] flex-none"
	/>

	<div class="relative inline-block">
		<Popup
			floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
			containerClasses="border rounded-lg shadow-lg bg-surface flex flex-col"
		>
			<svelte:fragment slot="button">
				<Button
					spacingSize="xs"
					size="xs"
					color="light"
					variant="border"
					nonCaptureEvent={true}
					disabled={webhookDisabled}
				>
					<span class="text-xs justify-center">Get webhook</span>
				</Button>
			</svelte:fragment>

			{#if !webhookDisabled}
				<Button
					size="xs"
					variant="contained"
					color="light"
					on:click={() => {
						copyToClipboard(url)
						isOpen = false
					}}
				>
					Url
					<Clipboard size={12} />
				</Button>

				{#if requestType !== 'get_path'}
					<Button
						size="xs"
						variant="contained"
						color="light"
						on:click={() => {
							copyToClipboard(JSON.stringify(args, null, 2))
							isOpen = false
						}}
					>
						Body
						<Clipboard size={12} />
					</Button>
				{/if}

				<Button
					size="xs"
					variant="contained"
					color="light"
					on:click={() => {
						copyToClipboard(JSON.stringify(headers(), null, 2))
						isOpen = false
					}}
				>
					Headers
					<Clipboard size={12} />
				</Button>

				<Button
					size="xs"
					variant="contained"
					color="light"
					on:click={() => {
						copyToClipboard(curlCode())
						isOpen = false
					}}
				>
					cUrl
					<Clipboard size={12} />
				</Button>
				<Button
					size="xs"
					variant="contained"
					color="light"
					on:click={() => {
						copyToClipboard(fetchCode())
						isOpen = false
					}}
				>
					Fetch
					<Clipboard size={12} />
				</Button>
			{/if}
		</Popup>
	</div>
</div>
