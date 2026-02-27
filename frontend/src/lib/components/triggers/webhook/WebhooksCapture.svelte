<script lang="ts">
	import Label from '../../Label.svelte'
	import bash from 'svelte-highlight/languages/bash'
	import { isObject } from '$lib/utils'
	import CopyableCodeBlock from '../../details/CopyableCodeBlock.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import { workspaceStore } from '$lib/stores'
	import { Url } from '$lib/components/common'
	import { fade } from 'svelte/transition'

	interface Props {
		isFlow?: boolean
		path?: string
		runnableArgs: any
		captureInfo?: CaptureInfo | undefined
		hasPreprocessor?: boolean
		captureLoading?: boolean
		oncaptureToggle?: (...args: any[]) => any
		onapplyArgs?: (...args: any[]) => any
		onupdateSchema?: (...args: any[]) => any
		onaddPreprocessor?: (...args: any[]) => any
		ontestWithArgs?: (...args: any[]) => any
	}

	let {
		isFlow = false,
		path = '',
		runnableArgs,
		captureInfo = undefined,
		hasPreprocessor = false,
		captureLoading = false,
		oncaptureToggle = undefined,
		onapplyArgs = undefined,
		onupdateSchema = undefined,
		onaddPreprocessor = undefined,
		ontestWithArgs = undefined
	}: Props = $props()

	let cleanedRunnableArgs = $derived(
		isObject(runnableArgs) && 'wm_trigger' in runnableArgs
			? Object.fromEntries(Object.entries(runnableArgs).filter(([key]) => key !== 'wm_trigger'))
			: runnableArgs
	)

	let captureUrl = $derived(
		`${location.origin}/api/w/${$workspaceStore}/capture_u/webhook/${
			isFlow ? 'flow' : 'script'
		}/${path}`
	)

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
		oncaptureToggle={oncaptureToggle}
		captureType="webhook"
		{captureLoading}
		onapplyArgs={onapplyArgs}
		onupdateSchema={onupdateSchema}
		onaddPreprocessor={onaddPreprocessor}
		ontestWithArgs={ontestWithArgs}
		{isFlow}
		{hasPreprocessor}
	>
		{#snippet description()}
			{#if captureInfo.active}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Send a POST request to the URL below to simulate a webhook event.
				</p>
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to webhook events on this test URL.
				</p>
			{/if}
		{/snippet}
		<Url label="Test URL" url={captureUrl} />

		<Label label="Example cURL" disabled={!captureInfo.active}>
			<CopyableCodeBlock code={captureCurlCode()} language={bash} disabled={!captureInfo.active} />
		</Label>
	</CaptureSection>
{/if}
