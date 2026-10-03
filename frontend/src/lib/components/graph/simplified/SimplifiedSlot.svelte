<script lang="ts">
	import { CircleDot } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import InsertModulePopover from '$lib/components/flows/map/InsertModulePopover.svelte'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import { getGraphContext } from '../graphContext'
	import { getSimplifiedEditContext } from './simplifiedContext'
	import type { InsertTarget } from './simplifiedLayout'

	let { insert }: { insert: InsertTarget } = $props()

	const edit = getSimplifiedEditContext()
	const moveManager = getGraphContext()?.moveManager

	let position = $derived({
		sourceId: insert.sourceId,
		targetId: insert.targetId,
		branch: insert.branch,
		index: insert.index
	})
	let moving = $derived(moveManager?.movingModuleId)
	// A step cannot be pasted inside itself
	let pasteBlocked = $derived(
		!!moving && (moveManager?.movingIds ?? [moving]).some((id) => insert.ancestors.includes(id))
	)
</script>

<!-- Centred on the node's own box, which the layout sizes to the button -->
<div class="nodrag nopan w-full h-full flex items-center justify-center">
	{#if moving}
		{#if !pasteBlocked}
			<Button
				variant="default"
				unifiedSize="2xs"
				iconOnly
				startIcon={{ icon: CircleDot }}
				title="Paste module"
				onClick={() => edit?.eventHandlers?.insert(position)}
			/>
		{/if}
	{:else}
		<InsertModulePopover
			disableAi={edit?.disableAi}
			allowTrigger={insert.allowTrigger}
			gutter={0}
			on:new={(e) =>
				edit?.eventHandlers?.insert({
					...position,
					kind: e.detail.kind,
					inlineScript: e.detail.inlineScript,
					agentPath: e.detail.agentPath
				})}
			on:pickScript={(e) =>
				edit?.eventHandlers?.insert({ ...position, script: e.detail, kind: e.detail.kind })}
			on:pickFlow={(e) => edit?.eventHandlers?.insert({ ...position, flow: e.detail })}
		>
			{#snippet trigger()}
				<InsertModuleButton title="Add step" />
			{/snippet}
		</InsertModulePopover>
	{/if}
</div>
