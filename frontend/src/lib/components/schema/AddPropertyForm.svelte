<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'
	import { Plus } from 'lucide-svelte'
	import TextInput from '../text_input/TextInput.svelte'

	let name: string = ''

	const dispatch = createEventDispatcher()

	function addField() {
		dispatch('add', { name })
		name = ''
	}
</script>

<div class="flex gap-2 whitespace-nowrap">
	<TextInput
		inputProps={{
			placeholder: 'Field name',
			onkeydown: (event) => {
				if (event.key === 'Enter') addField()
			}
		}}
		bind:value={name}
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
