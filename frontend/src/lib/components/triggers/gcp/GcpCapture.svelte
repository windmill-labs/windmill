<script lang="ts">
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'
	import { workspaceStore } from '$lib/stores'
	import { Url } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { base } from '$lib/base'

	interface Props {
		captureInfo?: CaptureInfo | undefined
		isValid?: boolean | undefined
		hasPreprocessor?: boolean
		isFlow?: boolean
		deliveryType?: 'push' | 'pull'
		captureLoading?: boolean
		triggerDeployed?: boolean
	}

	let {
		captureInfo = undefined,
		isValid = undefined,
		hasPreprocessor = false,
		isFlow = false,
		deliveryType = 'pull',
		captureLoading = false,
		triggerDeployed = false
	}: Props = $props()

	function getCaptureUrl(captureInfo: CaptureInfo | undefined) {
		if (!captureInfo) {
			return
		}
		return `${window.location.origin}${base}/api/w/${$workspaceStore}/capture_u/gcp/${
			captureInfo.isFlow ? 'flow' : 'script'
		}/${captureInfo.path}`
	}
</script>

{#if captureInfo}
	<CaptureSection
		captureType="gcp"
		disabled={isValid === false}
		{captureInfo}
		on:captureToggle
		on:applyArgs
		on:updateSchema
		on:addPreprocessor
		on:testWithArgs
		{hasPreprocessor}
		{isFlow}
		{captureLoading}
		displayAlert={triggerDeployed}
	>
		{#snippet description()}
			{#if captureInfo.active}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Listening to GCP Pub/Sub events...
				</p>
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to GCP Pub/Sub events.
				</p>
			{/if}
		{/snippet}

		{#if deliveryType === 'push'}
			{@const captureUrl = getCaptureUrl(captureInfo)}
			<Url url={captureUrl} label="Test URL" />
		{/if}
	</CaptureSection>
{/if}
