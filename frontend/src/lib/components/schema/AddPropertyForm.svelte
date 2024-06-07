<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	import { Button } from '../common'
	import { Plus } from 'lucide-svelte'

	let name: string = ''

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-row gap-2">
	<input bind:value={name} placeholder="Field name" />
	<Button
		variant="contained"
		color="dark"
		size="xs"
		startIcon={{ icon: Plus }}
		id="flow-editor-add-property"
		on:click={() => {
			dispatch('add', { name })
			name = ''
		}}
		disabled={!name}
	>
		Add field
	</Button>
</div>
