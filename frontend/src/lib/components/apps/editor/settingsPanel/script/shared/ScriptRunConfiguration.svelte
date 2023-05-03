<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { createEventDispatcher } from 'svelte'
	import ScriptSettingsSection from './ScriptSettingsSection.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	export let autoRefresh: boolean | undefined = false
	export let recomputeOnInputChanged: boolean | undefined = false
	export let canConfigureRecomputeOnInputChanged: boolean = true
	export let canConfigureRunOnStart: boolean = true

	const dispatch = createEventDispatcher()
</script>

{#if canConfigureRecomputeOnInputChanged || canConfigureRunOnStart}
	<ScriptSettingsSection title="Run configuration">
		<div class="flex flex-col gap-1">
			{#if autoRefresh !== undefined && canConfigureRunOnStart}
				<div class="flex items-center justify-between w-full">
					<div class="flex flex-row items-center gap-2 text-xs">
						Run on start and app refresh
						<Tooltip wrapperClass="flex">
							You may want to disable this so that the background runnable is only triggered by
							changes to other values or triggered by another computation on a button (See
							'Recompute Others')
						</Tooltip>
					</div>
					<Toggle
						bind:checked={autoRefresh}
						size="xs"
						on:change={() => {
							dispatch('updateAutoRefresh')
						}}
					/>
				</div>
			{/if}
			{#if recomputeOnInputChanged !== undefined && canConfigureRecomputeOnInputChanged}
				<div class="flex items-center justify-between w-full">
					<div class="flex flex-row items-center gap-2 text-xs">
						Recompute on any input changes
					</div>

					<Toggle
						bind:checked={recomputeOnInputChanged}
						size="xs"
						on:change={() => {
							dispatch('updateAutoRefresh')
						}}
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
