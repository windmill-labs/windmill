<script lang="ts">
	import type { AppEditorContext, DynamicInput } from '../../types'
	import { Badge, Button } from '$lib/components/common'
	import { faLink } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'

	export let input: DynamicInput

	const { connectingInput, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	function applyConnection() {
		if (!$connectingInput.opened && $connectingInput.input !== undefined) {
			input = $connectingInput.input

			// TODO: Check whether this is needed
			$selectedComponent = $selectedComponent

			$connectingInput = {
				opened: false,
				input: undefined
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
				input: undefined
			}
		}}
	>
		Connect this input to an ouput
	</Button>
{/if}
