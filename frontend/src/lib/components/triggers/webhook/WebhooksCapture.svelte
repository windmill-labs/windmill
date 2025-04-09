<script lang="ts">
	import Label from '../../Label.svelte'
	import bash from 'svelte-highlight/languages/bash'
	import ClipboardPanel from '../../details/ClipboardPanel.svelte'
	import { isObject } from '$lib/utils'
	import CopyableCodeBlock from '../../details/CopyableCodeBlock.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSectionV2.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import { workspaceStore } from '$lib/stores'

	export let isFlow: boolean = false
	export let path: string = ''
	export let runnableArgs: any
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined

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
