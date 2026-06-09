<script lang="ts" module>
	// The merged compare control: fork direction (deploy_to / update) and the
	// deployed↔draft comparison live in one toggle. `deploy_to`/`update` only
	// apply in a fork; a non-fork workspace has draft as the sole option (the
	// caller hides the toggle entirely in that case).
	export type CompareMode = 'deploy_to' | 'update' | 'draft'
</script>

<script lang="ts">
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { ArrowUp, ArrowDown, Pencil } from 'lucide-svelte'

	interface Props {
		selected: CompareMode
		isFork: boolean
		parentWorkspaceId?: string
		deployCount?: number
		updateCount?: number
		draftCount?: number
		disabled?: boolean
		onSelected: (v: CompareMode) => void
	}

	let {
		selected,
		isFork,
		parentWorkspaceId,
		deployCount = 0,
		updateCount = 0,
		draftCount = 0,
		disabled = false,
		onSelected
	}: Props = $props()

	// Append a count suffix only when there is something to act on, mirroring the
	// draft toggle (no "(0)" noise).
	function withCount(label: string, count: number): string {
		return count > 0 ? `${label} (${count})` : label
	}
</script>

<ToggleButtonGroup {disabled} {selected} onSelected={(v) => onSelected(v as CompareMode)} noWFull>
	{#snippet children({ item })}
		{#if isFork}
			<ToggleButton
				value="deploy_to"
				label={withCount(`Deploy to ${parentWorkspaceId}`, deployCount)}
				icon={ArrowUp}
				{item}
			/>
			<ToggleButton
				value="update"
				label={withCount('Update current', updateCount)}
				icon={ArrowDown}
				{item}
			/>
		{/if}
		<ToggleButton value="draft" label={`Deploy draft (${draftCount})`} icon={Pencil} {item} />
	{/snippet}
</ToggleButtonGroup>
