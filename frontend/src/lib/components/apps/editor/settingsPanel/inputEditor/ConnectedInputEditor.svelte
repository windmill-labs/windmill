<script lang="ts">
	import type { AppViewerContext } from '../../../types'
	import { Badge, Button } from '$lib/components/common'
	import { faArrowRight, faClose } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { ConnectedAppInput } from '../../../inputType'

	export let componentInput: ConnectedAppInput

	const { connectingInput, app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	function applyConnection() {
		if (
			!$connectingInput.opened &&
			$connectingInput.input !== undefined &&
			!componentInput.connection
		) {
			componentInput.connection = $connectingInput.input.connection
			$connectingInput = {
				opened: false,
				input: undefined,
				hoveredComponent: undefined
			}
			$app = $app
			$worldStore = $worldStore
		}
	}

	$: $connectingInput && applyConnection()
</script>

{#if componentInput.connection}
	<div class="flex justify-between w-full gap-1">
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
			$app = $app
		}}
	>
		Disconnect
	</Button>
{:else}
	<div class="flex justify-between w-full gap-1">
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
					input: undefined,
					hoveredComponent: undefined
				}
			}
		}}
	>
		Connect
	</Button>
{/if}
