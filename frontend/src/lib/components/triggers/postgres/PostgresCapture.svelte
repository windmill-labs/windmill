<script lang="ts">
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'

	export let captureInfo: CaptureInfo | undefined = undefined
	export let postgres_resource_path: string = ''
	export let hasPreprocessor: boolean = false
	export let isFlow: boolean = false
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
	>
		<svelte:fragment slot="description">
			{#if captureInfo.active}
				Listening to Postgres database events...
			{:else}
				Start capturing to listen to Postgres database events.
			{/if}
		</svelte:fragment>
	</CaptureSection>
{/if}
