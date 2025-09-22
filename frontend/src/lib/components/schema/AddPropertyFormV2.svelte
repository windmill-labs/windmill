<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'
	import { CornerDownLeft } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	let name: string = $state('')
	interface Props {
		customName?: string | undefined
		disabled?: boolean
		trigger?: import('svelte').Snippet
		noPopover?: boolean
	}

	let { customName = undefined, disabled = false, trigger, noPopover }: Props = $props()

	const dispatch = createEventDispatcher()

	function addField() {
		dispatch('add', { name })
		name = ''
	}

	const trigger_render = $derived(trigger)
</script>

{#snippet form({ close })}
	<div class="flex flex-row gap-2 p-2 rounded-md">
		<input
			bind:value={name}
			class="max-w-80"
			placeholder={`${customName ?? 'Field'} name`}
			onkeydown={(event) => {
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
{/snippet}

{#if noPopover}
	{@render form({ close: () => {} })}
{:else}
	<Popover closeButton={false} class="w-full" {disabled}>
		{#snippet trigger()}
			{@render trigger_render?.()}
		{/snippet}
		{#snippet content({ close })}
			{@render form({ close })}
		{/snippet}
	</Popover>
{/if}
