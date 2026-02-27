<script lang="ts">
	import { fade } from 'svelte/transition'
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'

	interface Props {
		captureInfo?: CaptureInfo | undefined
		isValid: boolean
		hasPreprocessor?: boolean
		isFlow?: boolean
		captureLoading?: boolean
		triggerDeployed?: boolean
		oncaptureToggle?: (...args: any[]) => any
		onapplyArgs?: (...args: any[]) => any
		onupdateSchema?: (...args: any[]) => any
		onaddPreprocessor?: (...args: any[]) => any
		ontestWithArgs?: (...args: any[]) => any
	}

	let {
		captureInfo = undefined,
		hasPreprocessor = false,
		isValid = false,
		isFlow = false,
		captureLoading = false,
		triggerDeployed = false,
		oncaptureToggle = undefined,
		onapplyArgs = undefined,
		onupdateSchema = undefined,
		onaddPreprocessor = undefined,
		ontestWithArgs = undefined
	}: Props = $props()
</script>

{#if captureInfo}
	<CaptureSection
		captureType="postgres"
		disabled={!isValid}
		{captureInfo}
		oncaptureToggle={oncaptureToggle}
		onapplyArgs={onapplyArgs}
		onupdateSchema={onupdateSchema}
		onaddPreprocessor={onaddPreprocessor}
		ontestWithArgs={ontestWithArgs}
		{hasPreprocessor}
		{isFlow}
		{captureLoading}
		displayAlert={triggerDeployed}
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
