<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import { Loader2 } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import RecomputeAllButton from './RecomputeAllButton.svelte'

	export let containerClass: string | undefined = undefined
	export let containerStyle: string | undefined = undefined

	const { connectingInput, bgRuns, recomputeAllContext } =
		getContext<AppViewerContext>('AppViewerContext')
</script>

<div
	class={twMerge('flex justify-center h-8 items-center gap-2', containerClass)}
	style={containerStyle}
>
	<div class="w-9">
		{#if $bgRuns.length > 0}
			<Popover notClickable>
				<span class="!text-2xs text-tertiary inline-flex gap-1 items-center"
					><Loader2 size={10} class="animate-spin" /> {$bgRuns.length}
				</span>
				<span slot="text"
					><div class="flex flex-col">
						{#each $bgRuns as bgRun}
							<div class="flex gap-2 items-center">
								<div class="text-2xs">{bgRun}</div>
							</div>
						{/each}
					</div></span
				>
			</Popover>
		{/if}
	</div>
	<div>
		{#if !$connectingInput.opened}
			<RecomputeAllButton
				interval={$recomputeAllContext.interval}
				componentNumber={$recomputeAllContext.componentNumber ?? 0}
				on:click={() => $recomputeAllContext.onRefresh?.()}
				on:setInter={(e) => {
					$recomputeAllContext.setInter?.(e.detail)
				}}
				refreshing={$recomputeAllContext.refreshing}
				progress={$recomputeAllContext.progress}
				loading={$recomputeAllContext.loading}
			/>
		{/if}
	</div>
</div>
