<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ScriptSettingsSection from './ScriptSettingsSection.svelte'

	export let autoRefresh: boolean | undefined = true
	export let recomputeOnInputChanged: boolean | undefined = true
	export let canConfigureRecomputeOnInputChanged: boolean = true
	export let canConfigureRunOnStart: boolean = true
</script>

{#if canConfigureRecomputeOnInputChanged || canConfigureRunOnStart}
	<ScriptSettingsSection title="Run configuration">
		<div class="flex flex-col gap-2">
			{#if autoRefresh !== undefined && canConfigureRunOnStart}
				<div class="flex items-center justify-between w-full">
					<div class="flex flex-row items-center gap-2 text-xs">
						Run on start and app refresh
						<Tooltip>
							You may want to disable this so that the background script is only triggered by
							changes to other values or triggered by another computation on a button (See
							'Recompute Others')
						</Tooltip>
					</div>
					<input type="checkbox" bind:checked={autoRefresh} class="!w-4 !h-4 !rounded-sm" />
				</div>
			{/if}
			{#if recomputeOnInputChanged !== undefined && canConfigureRecomputeOnInputChanged}
				<div class="flex items-center justify-between w-full">
					<div class="flex flex-row items-center gap-2 text-xs">
						Recompute on any input changes
					</div>
					<input
						type="checkbox"
						bind:checked={recomputeOnInputChanged}
						class="!w-4 !h-4 !rounded-sm"
					/>
				</div>
			{/if}
		</div>
	</ScriptSettingsSection>
{:else}
	<ScriptSettingsSection title="Run configuration">
		<div class="text-xs"> Triggerable component runs only when an interaction happens. </div>
	</ScriptSettingsSection>
{/if}
