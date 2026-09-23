<script lang="ts">
	import { Bell, BellOff } from 'lucide-svelte'

	import { Button } from '$lib/components/common'
	import Tooltip from '../Tooltip.svelte'
	import { toggleWorkspaceErrorHandler } from './errorHandlerToggle'

	interface Props {
		kind: 'script' | 'flow'
		scriptOrFlowPath: string
		errorHandlerMuted: boolean | undefined
		iconOnly?: boolean
	}

	let { kind, scriptOrFlowPath, errorHandlerMuted = $bindable(), iconOnly = true }: Props = $props()

	async function toggleErrorHandler(): Promise<void> {
		const next = await toggleWorkspaceErrorHandler(kind, scriptOrFlowPath, errorHandlerMuted)
		if (next !== undefined) errorHandlerMuted = next
	}
</script>

<Button
	title={errorHandlerMuted === undefined || !errorHandlerMuted
		? 'Disable workspace error handler for this script'
		: 'Enable workspace error handler for this script'}
	unifiedSize="md"
	on:click={toggleErrorHandler}
	variant="subtle"
	startIcon={{
		icon: errorHandlerMuted === undefined || !errorHandlerMuted ? Bell : BellOff
	}}
	{iconOnly}
>
	{#if errorHandlerMuted === undefined || !errorHandlerMuted}
		<div class="flex flex-row items-center">
			{#if !iconOnly}
				Mute
			{/if}
		</div>
	{:else}
		<div class="flex flex-row items-center">
			{#if !iconOnly}
				Unmute
			{/if}
		</div>
	{/if}
	<Tooltip>Disable workspace error handler, EE only</Tooltip>
</Button>
