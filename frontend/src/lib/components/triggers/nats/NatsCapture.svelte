<script lang="ts">
	import { fade } from 'svelte/transition'
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'

	interface Props {
		captureInfo?: CaptureInfo | undefined
		isValid?: boolean | undefined
		hasPreprocessor?: boolean
		isFlow?: boolean
		captureLoading?: boolean
	}

	let {
		captureInfo = undefined,
		isValid = undefined,
		hasPreprocessor = false,
		isFlow = false,
		captureLoading = false
	}: Props = $props()
</script>

{#if captureInfo}
	<CaptureSection
		captureType="nats"
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
	>
		{#snippet description()}
			{#if captureInfo.active}
				{#if captureInfo.connectionInfo?.connected}
					<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
						Listening to NATS messages...
					</p>
				{:else}
					<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
						Connecting to nats...
					</p>
				{/if}
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to NATS messages.
				</p>
			{/if}
		{/snippet}
	</CaptureSection>
{/if}
