<script lang="ts">
	import { Bell, BellOff } from 'lucide-svelte'

	import { Button } from '$lib/components/common'
	import { FlowService, ScriptService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import Tooltip from '../Tooltip.svelte'

	export let kind: 'script' | 'flow'
	export let scriptOrFlowPath: string
	export let errorHandlerMuted: boolean | undefined
	export let iconOnly: boolean = true

	async function toggleErrorHandler(): Promise<void> {
		if ($workspaceStore !== undefined) {
			try {
				if (kind === 'flow') {
					await FlowService.toggleWorkspaceErrorHandlerForFlow({
						workspace: $workspaceStore,
						path: scriptOrFlowPath,
						requestBody: {
							muted: !errorHandlerMuted
						}
					})
				} else {
					await ScriptService.toggleWorkspaceErrorHandlerForScript({
						workspace: $workspaceStore,
						path: scriptOrFlowPath,
						requestBody: {
							muted: !errorHandlerMuted
						}
					})
				}
			} catch (error) {
				sendUserToast(
					`Error while toggling Workspace Error Handler: ${error.body || error.message}`,
					true
				)
				return
			}
			errorHandlerMuted = !errorHandlerMuted
			sendUserToast(
				errorHandlerMuted ? 'Workspace error handler muted' : 'Workspace error handler active',
				false
			)
		}
	}
</script>

<Button
	title={errorHandlerMuted === undefined || !errorHandlerMuted
		? 'Disable workspace error handler for this script'
		: 'Enable workspace error handler for this script'}
	size="xs"
	on:click={toggleErrorHandler}
	color="light"
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
