<script lang="ts">
	import { fade } from 'svelte/transition'
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'

	interface Props {
		captureInfo?: CaptureInfo | undefined
		postgres_resource_path?: string
		hasPreprocessor?: boolean
		isFlow?: boolean
		captureLoading?: boolean
	}

	let {
		captureInfo = undefined,
		postgres_resource_path = '',
		hasPreprocessor = false,
		isFlow = false,
		captureLoading = false
	}: Props = $props()
</script>

{#if captureInfo}
	<CaptureSection
		captureType="postgres"
		disabled={!postgres_resource_path}
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
						Listening to Postgres database transactions...
					</p>
				{:else}
					<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
						Connecting to Postgres database...
					</p>
				{/if}
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to Postgres database transactions.
				</p>
			{/if}
		{/snippet}
	</CaptureSection>
{/if}
