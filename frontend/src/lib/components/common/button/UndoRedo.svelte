<script lang="ts">
	import { Redo, Undo } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { Button } from '..'

	export let undoProps: Record<string, any> = {}
	export let redoProps: Record<string, any> = {}
	const dispatch = createEventDispatcher()
</script>

<div class="flex">
	<Button
		title="Undo"
		variant="border"
		color="light"
		size="xs"
		btnClasses="!min-h-[30px] !rounded-r-none"
		on:click={() => dispatch('undo')}
		{...undoProps}
	>
		<Undo size={14} />
	</Button>
	<Button
		title="Redo"
		variant="border"
		color="light"
		size="xs"
		btnClasses="!min-h-[30px] !rounded-l-none !border-l-0"
		on:click={() => dispatch('redo')}
		{...redoProps}
	>
		<Redo size={14} />
	</Button>
</div>
