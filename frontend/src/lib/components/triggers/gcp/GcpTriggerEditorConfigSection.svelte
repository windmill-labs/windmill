<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { emptyStringTrimmed } from '$lib/utils'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Subsection from '$lib/components/Subsection.svelte'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let gcp_resource_path = ''

	$: isValid = !emptyStringTrimmed(gcp_resource_path)
</script>

<div>
	{#if showCapture && captureInfo}
		<CaptureSection
			captureType="gcp"
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
	<Section label="GCP" {headless}>
		<div class="flex flex-col w-full gap-4">
			<Subsection label="Connection setup">
				<div class="flex flex-col gap-3">
					<div class="flex flex-col gap-1">
						<p class="text-xs mb-1 text-tertiary">
							Select a gcp resource. <Required required={true} />
						</p>
						<ResourcePicker resourceType="gcp" bind:value={gcp_resource_path} />
						{#if isValid}
							<TestTriggerConnection kind="gcp" args={{ gcp_resource_path }} />
						{/if}
					</div>
				</div>
			</Subsection>
		</div>
	</Section>
</div>
