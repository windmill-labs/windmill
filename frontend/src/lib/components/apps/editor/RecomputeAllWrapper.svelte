<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import RecomputeAllComponents from './RecomputeAllComponents.svelte'
	import { dfs } from './appUtils'
	import { deepEqual } from 'fast-equals'
	import { Loader2 } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { twMerge } from 'tailwind-merge'

	export let containerClass: string | undefined = undefined
	export let containerStyle: string | undefined = undefined

	const { selectedComponent, app, connectingInput, allIdsInPath, bgRuns } =
		getContext<AppViewerContext>('AppViewerContext')

	let previousSelectedIds: string[] | undefined = undefined
	$: if (!deepEqual(previousSelectedIds, $selectedComponent)) {
		previousSelectedIds = $selectedComponent
		$allIdsInPath = ($selectedComponent ?? [])
			.flatMap((id) => dfs($app.grid, id, $app.subgrids ?? {}))
			.filter((x) => x != undefined) as string[]
	}
</script>

<div
	class={twMerge('w-full flex justify-center h-8 items-center gap-4', containerClass)}
	style={containerStyle}
>
	<div class="flex gap-2 items-center">
		<div>
			{#if !$connectingInput.opened}
				<RecomputeAllComponents />
			{/if}
		</div>
		{#if $bgRuns.length > 0}
			<Popover notClickable>
				<span class="!text-2xs text-tertiary inline-flex gap-1 items-center"
					><Loader2 size={10} class="animate-spin" /> {$bgRuns.length}
				</span>
				<span slot="text"
					><div class="flex flex-col">
						{#each $bgRuns as bgRun}
							<div class="flex gap-2 items-center">
								<div class="text-2xs text-tertiary">{bgRun}</div>
							</div>
						{/each}
					</div></span
				>
			</Popover>
		{:else}
			<span class="w-9" />
		{/if}
	</div>
</div>
