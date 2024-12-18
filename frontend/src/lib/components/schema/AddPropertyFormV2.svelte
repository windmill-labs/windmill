<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'
	import RoundIconButton from '../common/button/RoundIconButton.svelte'
	import { Plus, CornerDownLeft } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	let name: string = ''

	const dispatch = createEventDispatcher()

	function addField() {
		dispatch('add', { name })
		name = ''
	}

	let open: boolean = false
</script>

<div class="flex flex-row gap-2">
	<Popover closeButton={false} bind:open>
		<svelte:fragment slot="trigger">
			<RoundIconButton>
				<Plus size={16} />
			</RoundIconButton>
		</svelte:fragment>
		<svelte:fragment slot="content">
			<div class="flex flex-row gap-2 p-2 rounded-md">
				<input
					bind:value={name}
					placeholder="Field name"
					on:keydown={(event) => {
						if (event.key === 'Enter') {
							addField()
							open = false
						}
					}}
				/>
				<Button
					variant="contained"
					color="dark"
					size="xs"
					id="flow-editor-add-property"
					on:click={addField}
					disabled={!name}
					shortCut={{ Icon: CornerDownLeft, withoutModifier: true }}
				>
					Add field
				</Button>
			</div>
		</svelte:fragment>
	</Popover>
</div>
