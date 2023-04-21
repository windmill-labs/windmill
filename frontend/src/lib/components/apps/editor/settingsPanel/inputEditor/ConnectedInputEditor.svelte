<script lang="ts">
	import type { AppViewerContext } from '../../../types'
	import { Badge, Button } from '$lib/components/common'
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { ConnectedAppInput } from '../../../inputType'
	import { Plug } from 'lucide-svelte'

	export let componentInput: ConnectedAppInput

	const { connectingInput, app } = getContext<AppViewerContext>('AppViewerContext')

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
		}
	}

	$: $connectingInput && applyConnection()
</script>

{#if componentInput.connection}
	<div class="flex justify-between w-full items-center">
		<span class="text-xs">Connection</span>
		<div>
			<Badge color="indigo">{componentInput.connection.componentId}</Badge>
			<Badge color="indigo">{componentInput.connection.path}</Badge>
		</div>
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
	<Button
		size="xs"
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
		<div class="flex flex-row gap-1 items-center">
			<span>Connect</span>
			<Plug size={14} />
		</div>
	</Button>
{/if}
