<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false

	$: isValid = true
</script>

<div>
	{#if showCapture && captureInfo}
		<CaptureSection
			captureType="mqtt"
			disabled={!isValid}
			{captureInfo}
			on:captureToggle
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			on:testWithArgs
			bind:captureTable
		/>
	{/if}
	<Section label="Mqtt" {headless}>
		<div class="flex flex-col w-full gap-4">
			
		</div>
	</Section>
</div>