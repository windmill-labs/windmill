<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'
	import { CornerDownLeft } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	let name: string = ''
	export let customName: string | undefined = undefined
	export let disabled: boolean = false
	export let placeholder: string | undefined = undefined

	const dispatch = createEventDispatcher()

	function addField() {
		dispatch('add', { name })
		name = ''
	}
</script>

<Popover closeButton={false} class="w-full" {disabled}>
	<svelte:fragment slot="trigger">
		<slot name="trigger" />
	</svelte:fragment>
	<svelte:fragment slot="content" let:close>
		<div class="flex flex-row gap-2 p-2 rounded-md">
			<input
				bind:value={name}
				placeholder={placeholder ?? `${customName ?? 'Field'} name`}
				on:keydown={(event) => {
					if (event.key === 'Enter') {
						addField()
						close()
					}
				}}
				{disabled}
			/>
			<Button
				variant="contained"
				color="dark"
				size="xs"
				id="flow-editor-add-property"
				on:click={() => {
					addField()
					close()
				}}
				disabled={!name || disabled}
				shortCut={{ Icon: CornerDownLeft, withoutModifier: true }}
			>
				Add {customName ? customName.toLowerCase() : 'field'}
			</Button>
		</div>
	</svelte:fragment>
</Popover>
