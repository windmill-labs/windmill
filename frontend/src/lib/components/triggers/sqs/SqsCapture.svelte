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
		triggerDeployed?: boolean
		oncaptureToggle?: (...args: any[]) => any
		onapplyArgs?: (...args: any[]) => any
		onupdateSchema?: (...args: any[]) => any
		onaddPreprocessor?: (...args: any[]) => any
		ontestWithArgs?: (...args: any[]) => any
	}

	let {
		captureInfo = undefined,
		isValid = undefined,
		hasPreprocessor = false,
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
		captureType="sqs"
		disabled={isValid === false}
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
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Listening to SQS messages...
				</p>
			{:else}
				<p in:fade={{ duration: 100, delay: 50 }} out:fade={{ duration: 50 }}>
					Start capturing to listen to SQS messages.
				</p>
			{/if}
		{/snippet}
	</CaptureSection>
{/if}
