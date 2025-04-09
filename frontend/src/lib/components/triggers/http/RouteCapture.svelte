<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Label from '$lib/components/Label.svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import { bash } from 'svelte-highlight/languages'
	// import { page } from '$app/stores'
	import { base } from '$lib/base'
	import type { CaptureInfo } from '../CaptureSectionV2.svelte'
	import CaptureSection from '../CaptureSectionV2.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import ClipboardPanel from '../../details/ClipboardPanel.svelte'
	import { isObject } from '$lib/utils'

	export let route_path: string | undefined
	export let http_method: 'get' | 'post' | 'put' | 'patch' | 'delete' | undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let captureTable: CaptureTable | undefined = undefined
	export let runnableArgs: any = {}
	export let isValid: boolean = false

	$: captureURL = `${location.origin}${base}/api/w/${$workspaceStore}/capture_u/http/${
		captureInfo?.isFlow ? 'flow' : 'script'
	}/${captureInfo?.path.replaceAll('/', '.')}/${route_path}`
	$: cleanedRunnableArgs =
		isObject(runnableArgs) && 'wm_trigger' in runnableArgs
			? Object.fromEntries(Object.entries(runnableArgs).filter(([key]) => key !== 'wm_trigger'))
			: runnableArgs

	$: !http_method && (http_method = 'post')
	$: route_path === undefined && (route_path = '')
</script>

{#if captureInfo}
	<CaptureSection
		captureType="http"
		disabled={!isValid}
		{captureInfo}
		on:captureToggle
		on:applyArgs
		on:updateSchema
		on:addPreprocessor
		on:testWithArgs
		bind:captureTable
	>
		<Label label="URL">
			<ClipboardPanel content={captureURL} disabled={!captureInfo.active} />
		</Label>

		<Label label="Example cUrl">
			<CopyableCodeBlock
				disabled={!captureInfo.active}
				code={`curl \\
-X ${(http_method ?? 'post').toUpperCase()} ${captureURL} \\
-H 'Content-Type: application/json' \\
-d '${JSON.stringify(cleanedRunnableArgs ?? {}, null, 2)}'`}
				language={bash}
			/>
		</Label>
	</CaptureSection>
{/if}
