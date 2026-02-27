<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Label from '$lib/components/Label.svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import { bash } from 'svelte-highlight/languages'
	// import { page } from '$app/state'
	import { base } from '$lib/base'
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'
	import { isObject } from '$lib/utils'
	import { Url } from '$lib/components/common'
	import { fade } from 'svelte/transition'

	interface Props {
		route_path: string | undefined
		http_method: 'get' | 'post' | 'put' | 'patch' | 'delete' | undefined
		captureInfo?: CaptureInfo | undefined
		runnableArgs?: any
		isValid?: boolean | undefined
		hasPreprocessor?: boolean
		isFlow?: boolean
		captureLoading?: boolean
		oncaptureToggle?: (...args: any[]) => any
		onapplyArgs?: (...args: any[]) => any
		onupdateSchema?: (...args: any[]) => any
		onaddPreprocessor?: (...args: any[]) => any
		ontestWithArgs?: (...args: any[]) => any
	}

	let {
		route_path,
		http_method,
		captureInfo = undefined,
		runnableArgs = {},
		isValid = undefined,
		hasPreprocessor = false,
		isFlow = false,
		captureLoading = false,
		oncaptureToggle = undefined,
		onapplyArgs = undefined,
		onupdateSchema = undefined,
		onaddPreprocessor = undefined,
		ontestWithArgs = undefined
	}: Props = $props()

	let captureURL = $derived(
		`${location.origin}${base}/api/w/${$workspaceStore}/capture_u/http/${
			captureInfo?.isFlow ? 'flow' : 'script'
		}/${captureInfo?.path.replaceAll('/', '.')}/${route_path ?? ''}`
	)
	let cleanedRunnableArgs = $derived(
		isObject(runnableArgs) && 'wm_trigger' in runnableArgs
			? Object.fromEntries(Object.entries(runnableArgs).filter(([key]) => key !== 'wm_trigger'))
			: runnableArgs
	)
</script>

{#if captureInfo}
	<CaptureSection
		captureType="http"
		disabled={isValid === false}
		{captureInfo}
		{captureLoading}
		oncaptureToggle={oncaptureToggle}
		onapplyArgs={onapplyArgs}
		onupdateSchema={onupdateSchema}
		onaddPreprocessor={onaddPreprocessor}
		ontestWithArgs={ontestWithArgs}
		{hasPreprocessor}
		{isFlow}
	>
		{#snippet description()}
			{#if captureInfo.active}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Send a POST request to the URL below to simulate a http event.
				</p>
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to HTTP requests on this test URL.
				</p>
			{/if}
		{/snippet}

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
