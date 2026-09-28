<script lang="ts">
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import { ChevronsDownUp, ChevronsUpDown } from 'lucide-svelte'
	import { storeLocalSetting } from '$lib/utils'

	interface Props {
		treeView: boolean
		collapseAll: boolean
		// Local setting the tree view choice is persisted under.
		settingName: string
		// The tree is showing every node open (a filter is active), so Expand/Collapse all
		// would have no effect.
		forceExpanded?: boolean
		class?: string
	}

	let {
		treeView = $bindable(),
		collapseAll = $bindable(),
		settingName,
		forceExpanded = false,
		class: clazz = ''
	}: Props = $props()
</script>

<div class="flex items-center gap-2 {clazz}">
	<Toggle
		size="xs"
		bind:checked={treeView}
		on:change={(e) => storeLocalSetting(settingName, e.detail ? 'true' : undefined)}
		options={{ right: 'Tree view' }}
	/>
	{#if treeView && !forceExpanded}
		<Button
			unifiedSize="sm"
			variant="subtle"
			on:click={() => (collapseAll = !collapseAll)}
			startIcon={{ icon: collapseAll ? ChevronsUpDown : ChevronsDownUp }}
		>
			{collapseAll ? 'Expand all' : 'Collapse all'}
		</Button>
	{/if}
</div>
