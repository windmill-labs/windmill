<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	import { Button } from '../common'
	import { Plus } from 'lucide-svelte'

	let name: string = ''
	let type: string = 'string'

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-row gap-2">
	<input bind:value={name} placeholder="Argument name" />
	<Button
		variant="contained"
		color="dark"
		size="xs"
		startIcon={{ icon: Plus }}
		id="flow-editor-add-property"
		on:click={() => {
			dispatch('add', { name, type })
		}}
		disabled={!name}
	>
		Add Argument
	</Button>
</div>
