<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import RecomputeAllComponents from './RecomputeAllComponents.svelte'
	import type { Policy } from '$lib/gen'
	import { dfs } from './appUtils'
	import { deepEqual } from 'fast-equals'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { twMerge } from 'tailwind-merge'

	export let policy: Policy
	export let displayTitle: boolean = true
	export let displayRecompute: boolean = true
	export let displayAuthor: boolean = true
	export let titleOverride: string | undefined = undefined
	export let containerClass: string | undefined = undefined
	export let containerStyle: string | undefined = undefined

	const { selectedComponent, app, connectingInput, summary, allIdsInPath, bgRuns } =
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
	class={twMerge(
		'w-full sticky top-0 flex justify-between border-b h-8 items-center gap-4',
		$connectingInput?.opened ? '' : 'bg-surface',
		containerClass
	)}
	style={containerStyle}
>
	{#if displayTitle}
		<h3 class="truncate">{titleOverride ? titleOverride : $summary}</h3>
	{/if}
	{#if displayRecompute}
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
	{/if}
	<div class="flex text-2xs gap-8 items-center">
		{#if displayAuthor}
			<div>
				{policy.on_behalf_of ? `Author ${policy.on_behalf_of_email}` : ''}
				<Tooltip>
					The scripts will be run on behalf of the author and a tight policy ensure security about
					the possible inputs of the runnables.
				</Tooltip>
			</div>
		{/if}
	</div>
</div>
