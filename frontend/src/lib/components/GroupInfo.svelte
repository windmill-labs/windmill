<script lang="ts">
	import { GroupService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Popover from './Popover.svelte'
	import { untrack } from 'svelte'

	interface Props {
		name: string
	}

	let { name }: Props = $props()

	let members: string[] | undefined = $state([])

	async function loadMembers() {
		members = (await GroupService.getGroup({ workspace: $workspaceStore!, name })).members
	}
	$effect(() => {
		$workspaceStore && untrack(() => loadMembers())
	})
</script>

{#if members}
	<Popover
		><div class="inline-flex gap-1 items-end"
			><span class="text-tertiary text-xs">({members.length})</span>
			<div class="max-w-xs truncate"
				><span class="text-tertiary text-xs">{members?.join(', ')}</span></div
			></div
		>
		{#snippet text()}
			<span>{members?.join(', ')}</span>
		{/snippet}</Popover
	>
{/if}
