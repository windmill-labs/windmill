<script lang="ts">
	import Popover from './Popover.svelte'

	export let members: string[]
</script>

<Popover
	><div class="inline-flex gap-1 items-end"
		><span class="text-tertiary text-xs mb-0.5">({members.length})</span>
		<div class="max-w-xs truncate"
			><span class="text-tertiary text-xs">{members?.join(', ')}</span></div
		></div
	>
	<span slot="text">{members?.join(', ')}</span></Popover
>
