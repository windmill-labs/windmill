<script lang="ts">
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'
	import { Url } from '$lib/components/common'
	import { fade } from 'svelte/transition'

	interface Props {
		captureInfo?: CaptureInfo | undefined
		isValid?: boolean | undefined
		hasPreprocessor?: boolean
		isFlow?: boolean
		captureLoading?: boolean
		subscriptionName?: string | undefined
	}

	let {
		captureInfo = undefined,
		isValid = undefined,
		hasPreprocessor = false,
		isFlow = false,
		captureLoading = false,
		subscriptionName = undefined
	}: Props = $props()

	// Mirror the backend suffix in set_azure_trigger_config: truncate to 39 then
	// append "-wm-capture". Azure rule: [A-Za-z0-9-]{3,50}.
	const captureSubscriptionName = $derived(
		subscriptionName
			? `${subscriptionName.length > 39 ? subscriptionName.slice(0, 39) : subscriptionName}-wm-capture`
			: undefined
	)
</script>

{#if captureInfo}
	<CaptureSection
		captureType="azure"
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
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Listening to Azure Event Grid events...
				</p>
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to Azure Event Grid events.
				</p>
			{/if}
		{/snippet}
		{#if captureSubscriptionName}
			<Url label="Test subscription name" url={captureSubscriptionName} />
		{/if}
	</CaptureSection>
{/if}
