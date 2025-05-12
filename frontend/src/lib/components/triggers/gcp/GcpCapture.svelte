<script lang="ts">
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'
	import { workspaceStore } from '$lib/stores'
	import { Url } from '$lib/components/common'
	import { fade } from 'svelte/transition'

	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean | undefined = undefined
	export let hasPreprocessor: boolean = false
	export let isFlow: boolean = false
	export let deliveryType: 'push' | 'pull' = 'pull'
	export let baseEndpoint: string = ''

	function getCaptureUrl(captureInfo: CaptureInfo | undefined) {
		if (!captureInfo) {
			return
		}
		return `${baseEndpoint}/api/w/${$workspaceStore}/capture_u/gcp/${
			captureInfo?.isFlow ? 'flow' : 'script'
		}${captureInfo?.path} `
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
	>
		<svelte:fragment slot="description">
			{#if captureInfo.active}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Listening to GCP Pub/Sub events...
				</p>
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to GCP Pub/Sub events.
				</p>
			{/if}
		</svelte:fragment>

		{#if deliveryType === 'push'}
			{@const captureUrl = getCaptureUrl(captureInfo)}
			<Url url={captureUrl} label="Test URL" />
		{/if}
	</CaptureSection>
{/if}
