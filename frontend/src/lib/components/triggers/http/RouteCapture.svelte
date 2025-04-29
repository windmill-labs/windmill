<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Label from '$lib/components/Label.svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import { bash } from 'svelte-highlight/languages'
	// import { page } from '$app/stores'
	import { base } from '$lib/base'
	import type { CaptureInfo } from '../CaptureSectionV2.svelte'
	import CaptureSection from '../CaptureSectionV2.svelte'
	import { isObject } from '$lib/utils'
	import { Url } from '$lib/components/common'

	export let route_path: string | undefined
	export let http_method: 'get' | 'post' | 'put' | 'patch' | 'delete' | undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let runnableArgs: any = {}
	export let isValid: boolean | undefined = undefined
	export let hasPreprocessor: boolean = false
	export let isFlow: boolean = false

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
		disabled={isValid === false}
		{captureInfo}
		on:captureToggle
		on:applyArgs
		on:updateSchema
		on:addPreprocessor
		on:testWithArgs
		{hasPreprocessor}
		{isFlow}
	>
		<svelte:fragment slot="description">
			{#if captureInfo.active}
				Send a POST request to the URL below to simulate a http event.
			{:else}
				Start capturing to listen to HTTP requests on this test URL.
			{/if}
		</svelte:fragment>

		<Url url={captureURL} label="Test URL" />

		<Label label="Example cURL" disabled={!captureInfo.active}>
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
