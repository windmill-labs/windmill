<script lang="ts">
	import Label from '../../Label.svelte'
	import bash from 'svelte-highlight/languages/bash'
	import { isObject } from '$lib/utils'
	import CopyableCodeBlock from '../../details/CopyableCodeBlock.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import { workspaceStore } from '$lib/stores'
	import { Url } from '$lib/components/common'

	export let isFlow: boolean = false
	export let path: string = ''
	export let runnableArgs: any
	export let captureInfo: CaptureInfo | undefined = undefined
	export let hasPreprocessor: boolean = false

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

{#if captureInfo}
	<CaptureSection
		{captureInfo}
		disabled={false}
		on:captureToggle
		captureType="webhook"
		on:applyArgs
		on:updateSchema
		on:addPreprocessor
		on:testWithArgs
		{isFlow}
		{hasPreprocessor}
	>
		<svelte:fragment slot="description">
			{#if captureInfo.active}
				Send a POST request to the URL below to simulate a webhook event.
			{:else}
				Start capturing to listen to webhook events on this test URL.
			{/if}
		</svelte:fragment>
		<Url label="Test URL" url={captureUrl} />

		<Label label="Example cURL" disabled={!captureInfo.active}>
			<CopyableCodeBlock code={captureCurlCode()} language={bash} disabled={!captureInfo.active} />
		</Label>
	</CaptureSection>
{/if}
