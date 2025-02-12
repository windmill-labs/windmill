<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import MultiSelect from 'svelte-multiselect'
	import { X } from 'lucide-svelte'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let topics: string[] = []
	$: isValid = topics.length > 0
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
		<Section label="topics">
			<p class="text-xs text-tertiary mb-2"
				>Enter which topics you want to subscribe to<Required required={true} />
			</p>
			<MultiSelect
				noMatchingOptionsMsg=""
				createOptionMsg={null}
				duplicates={false}
				options={[]}
				allowUserOptions="append"
				bind:selected={topics}
				ulOptionsClass={'!bg-surface !text-sm'}
				ulSelectedClass="!text-sm"
				outerDivClass="!bg-surface !min-h-[38px] !border-[#d1d5db]"
				placeholder="Enter topics"
				--sms-options-margin="4px"
				--sms-open-z-index="100"
				disabled={!can_write}
			>
				<svelte:fragment slot="remove-icon">
					<div class="hover:text-primary p-0.5">
						<X size={12} />
					</div>
				</svelte:fragment>
			</MultiSelect>
		</Section>
	</Section>
</div>
