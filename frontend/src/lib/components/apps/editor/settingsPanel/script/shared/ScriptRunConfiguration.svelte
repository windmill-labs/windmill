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
	<ScriptSettingsSection title="Triggers">
		<div class="flex flex-col gap-2 mb-4">
			{#if autoRefresh !== undefined && canConfigureRunOnStart}
				<div class="flex items-center justify-between w-full gap-1">
					<div class="flex flex-row items-center gap-2 text-xs mb-0.5">
						Run on start and app refresh
						<Tooltip>
							You may want to disable this so that the background runnable is only triggered by
							changes to other values or triggered by another computation on a button (See 'Trigger
							Runnables')
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
		<slot />
	</ScriptSettingsSection>
{:else}
	<ScriptSettingsSection title="Triggers">
		<slot />
	</ScriptSettingsSection>
{/if}
