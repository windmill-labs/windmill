<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'

	export let url: string | undefined
	export let url_runnable_args: Record<string, unknown> | undefined
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	let postgres_resource_path: string = ''
</script>

<div>
	{#if showCapture && captureInfo}
		<CaptureSection
			disabled={!isValid}
			on:captureToggle
			captureType="postgres"
			{captureInfo}
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			on:testWithArgs
			bind:captureTable
		/>
	{/if}
	<Section label="Postgres" {headless}>
		<div class="mb-2">
			<ToggleButtonGroup
				selected={url?.startsWith('$') ? 'runnable' : 'static'}
				on:selected={(ev) => {
					url = ev.detail === 'runnable' ? '$script:' : ''
					url_runnable_args = {}
				}}
			>
				<ToggleButton value="static" label="Static URL" />
				<ToggleButton value="runnable" label="Runnable result as URL" />
			</ToggleButtonGroup>
		</div>
		<TestTriggerConnection kind="postgres" args={{ postgres_resource_path }} />
	</Section>
</div>
