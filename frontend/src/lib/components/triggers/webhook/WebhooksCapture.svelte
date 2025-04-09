<script lang="ts">
	import Label from '../../Label.svelte'
	import bash from 'svelte-highlight/languages/bash'
	import ClipboardPanel from '../../details/ClipboardPanel.svelte'
	import { isObject } from '$lib/utils'
	import { base } from '$lib/base'
	import TriggerTokens from '../TriggerTokens.svelte'
	import { workspaceStore, userStore } from '$lib/stores'
	import UserSettings from '../../UserSettings.svelte'
	import { generateRandomString } from '$lib/utils'
	import CopyableCodeBlock from '../../details/CopyableCodeBlock.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSectionV2.svelte'
	import CaptureTable from '../CaptureTable.svelte'

	export let isFlow: boolean = false
	export let path: string = ''
	export let hash: string | undefined = undefined
	export let token: string = ''
	export let runnableArgs: any
	export let triggerTokens: TriggerTokens | undefined = undefined
	export let scopes: string[] = []
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined

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
	let userSettings: UserSettings

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

	let webhookType: 'async' | 'sync' = 'async'
	let requestType: 'hash' | 'path' | 'get_path' = isFlow ? 'path' : 'path'

	$: if (webhookType === 'async' && requestType === 'get_path') {
		requestType = hash ? 'hash' : 'path'
	}

	$: cleanedRunnableArgs =
		isObject(runnableArgs) && 'wm_trigger' in runnableArgs
			? Object.fromEntries(Object.entries(runnableArgs).filter(([key]) => key !== 'wm_trigger'))
			: runnableArgs

	let captureUrl = `${location.origin}/api/w/${$workspaceStore}/capture_u/webhook/${
		isFlow ? 'flow' : 'script'
	}/${path}`

	function captureCurlCode() {
		return `curl \\
-X POST ${captureUrl} \\
-H 'Content-Type: application/json' \\
-d '${JSON.stringify(cleanedRunnableArgs ?? {}, null, 2)}'`
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

{#if captureInfo}
	<CaptureSection
		{captureInfo}
		disabled={false}
		on:captureToggle
		captureType="webhook"
		bind:captureTable
		on:applyArgs
		on:updateSchema
		on:addPreprocessor
		on:testWithArgs
	>
		<Label label="URL">
			<ClipboardPanel content={captureUrl} disabled={!captureInfo.active} />
		</Label>

		<Label label="Example cURL">
			<CopyableCodeBlock code={captureCurlCode()} language={bash} disabled={!captureInfo.active} />
		</Label>
	</CaptureSection>
{/if}
