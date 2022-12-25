<script lang="ts">
	import type { AppEditorContext } from '../../../types'
	import { Badge, Button } from '$lib/components/common'
	import { faArrowRight, faClose } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../../inputType'

	export let componentInput: AppInput

	const { connectingInput } = getContext<AppEditorContext>('AppEditorContext')

	function applyConnection() {
		if (
			!$connectingInput.opened &&
			$connectingInput.input !== undefined &&
			componentInput.type === 'connected' &&
			!componentInput.connection
		) {
			componentInput.connection = $connectingInput.input.connection
			$connectingInput = {
				opened: false,
				input: undefined
			}
		}
	}

	$: $connectingInput && applyConnection()
</script>

{#if componentInput.type === 'connected'}
	{#if componentInput.connection}
		<div class="flex justify-between w-full">
			<span class="text-xs">Status</span>
			<Badge color="green">Connected</Badge>
		</div>
		<div class="flex justify-between w-full">
			<span class="text-xs">Component</span>
			<Badge color="indigo">{componentInput.connection.componentId}</Badge>
		</div>
		<div class="flex justify-between w-full">
			<span class="text-xs">Path</span>
			<Badge color="indigo">{componentInput.connection.path}</Badge>
		</div>
		<Button
			size="xs"
			startIcon={{ icon: faClose }}
			color="red"
			variant="border"
			on:click={() => {
				if (componentInput.type === 'connected') {
					componentInput.connection = undefined
				}
			}}
		>
			Disconnect
		</Button>
	{:else}
		<div class="flex justify-between w-full">
			<span class="text-xs">Status</span>
			<Badge color="yellow">Not connected</Badge>
		</div>
		<Button
			size="xs"
			endIcon={{ icon: faArrowRight }}
			color="blue"
			on:click={() => {
				if (componentInput.type === 'connected') {
					$connectingInput = {
						opened: true,
						input: undefined
					}
				}
			}}
		>
			Connect
		</Button>
	{/if}
{/if}
