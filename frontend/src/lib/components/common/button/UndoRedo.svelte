<script lang="ts">
	import { Redo, Undo } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { Button } from '..'

	interface Props {
		undoProps?: Record<string, any>;
		redoProps?: Record<string, any>;
	}

	let { undoProps = {}, redoProps = {} }: Props = $props();
	const dispatch = createEventDispatcher()
</script>

<div class="flex">
	<Button
		title="Undo"
		variant="default"
		btnClasses="!min-h-[30px] !rounded-r-none"
		on:click={() => dispatch('undo')}
		startIcon={{ icon: Undo }}
		iconOnly
		{...undoProps}
	></Button>
	<Button
		title="Redo"
		variant="default"
		btnClasses="!min-h-[30px] !rounded-l-none !border-l-0"
		on:click={() => dispatch('redo')}
		startIcon={{ icon: Redo }}
		iconOnly
		{...redoProps}
	></Button>
</div>
