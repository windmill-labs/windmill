<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false

</script>

<div>
	{#if showCapture && captureInfo}
		<CaptureSection
			disabled={!isValid}
			on:captureToggle
			captureType="sqs"
			{captureInfo}
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			on:testWithArgs
			bind:captureTable
		/>
	{/if}
	<Section label="Sqs" {headless}>
		{#if isValid}
			<TestTriggerConnection kind="sqs" args={{  }} />
		{/if}
	</Section>
</div>
