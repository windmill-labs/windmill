<script lang="ts">
	import type { AppEditorContext, DynamicInput, InputType } from '../../types'
	import { Badge, Button } from '$lib/components/common'
	import { faLink } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'

	export let input: DynamicInput<InputType, any>

	const { connectingInput, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	function applyConnection() {
		if (!$connectingInput.opened && $connectingInput.input !== undefined) {
			input.id = $connectingInput.input.id
			input.name = $connectingInput.input.name

			$connectingInput = {
				opened: false,
				input: undefined,
				sourceName: undefined
			}
		}
	}

	$: $connectingInput && applyConnection()
</script>

{#if input.id && input.name}
	<div class="flex justify-between w-full">
		<span class="text-xs font-bold">Status</span>
		<Badge color="green">Connected</Badge>
	</div>
	<div class="flex justify-between w-full">
		<span class="text-xs font-bold">Component</span>
		<Badge color="indigo">{input.id}</Badge>
	</div>
	<div class="flex justify-between w-full">
		<span class="text-xs font-bold">Field name</span>
		<Badge color="indigo">{input.name}</Badge>
	</div>
	<Button
		size="xs"
		startIcon={{ icon: faLink }}
		color="red"
		on:click={() => {
			input.id = undefined
			input.name = undefined
		}}
	>
		Clear connection
	</Button>
{:else}
	<div class="flex justify-between w-full">
		<span class="text-xs font-bold">Status</span>
		<Badge color="dark-yellow">Not connected</Badge>
	</div>
	<Button
		size="xs"
		startIcon={{ icon: faLink }}
		color="dark"
		on:click={() => {
			$connectingInput = {
				opened: true,
				input: undefined,
				sourceName: input.name
			}
		}}
	>
		Connect this input to an output
	</Button>
{/if}
