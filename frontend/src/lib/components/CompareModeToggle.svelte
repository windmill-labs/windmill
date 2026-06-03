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
		draftCount?: number
		disabled?: boolean
		onSelected: (v: CompareMode) => void
	}

	let {
		selected,
		isFork,
		parentWorkspaceId,
		draftCount = 0,
		disabled = false,
		onSelected
	}: Props = $props()
</script>

<ToggleButtonGroup {disabled} {selected} onSelected={(v) => onSelected(v as CompareMode)} noWFull>
	{#snippet children({ item })}
		{#if isFork}
			<ToggleButton value="deploy_to" label="Deploy to {parentWorkspaceId}" icon={ArrowUp} {item} />
			<ToggleButton value="update" label="Update current" icon={ArrowDown} {item} />
		{/if}
		<ToggleButton
			value="draft"
			label={draftCount > 0 ? `Deployed ↔ draft (${draftCount})` : 'Deployed ↔ draft'}
			icon={Pencil}
			{item}
		/>
	{/snippet}
</ToggleButtonGroup>
