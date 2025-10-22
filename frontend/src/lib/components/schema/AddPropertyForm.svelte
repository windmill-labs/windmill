<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'
	import { Plus } from 'lucide-svelte'

	let name: string = ''

	const dispatch = createEventDispatcher()

	function addField() {
		dispatch('add', { name })
		name = ''
	}
</script>

<div class="flex flex-row gap-2">
	<input
		bind:value={name}
		placeholder="Field name"
		on:keydown={(event) => {
			if (event.key === 'Enter') {
				addField()
			}
		}}
	/>
	<Button
		variant="accent"
		size="xs"
		startIcon={{ icon: Plus }}
		id="flow-editor-add-property"
		on:click={addField}
		disabled={!name}
	>
		Add field
	</Button>
</div>
