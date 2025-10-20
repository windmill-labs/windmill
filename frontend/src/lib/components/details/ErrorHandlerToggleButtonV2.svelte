<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'

	import { FlowService, ScriptService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'

	export let kind: 'script' | 'flow'
	export let scriptOrFlowPath: string
	export let errorHandlerMuted: boolean | undefined
	export let textDisabled = false
	let toggleState = errorHandlerMuted

	export let color: 'nord' | 'red' | 'blue' | undefined = undefined

	async function toggleErrorHandler(): Promise<void> {
		toggleState = !toggleState
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
				toggleState = false
				return
			}
			errorHandlerMuted = !errorHandlerMuted
			toggleState = errorHandlerMuted
			sendUserToast(
				errorHandlerMuted ? 'Workspace error handler muted' : 'Workspace error handler active',
				false
			)
		}
	}
</script>

<Toggle
	textClass="font-medium"
	{color}
	size="xs"
	checked={toggleState}
	on:change={toggleErrorHandler}
	options={{
		right: 'Mute',
		rightTooltip: 'Disable workspace error handler, EE only',
		rightDocumentationLink:
			'https://www.windmill.dev/docs/core_concepts/error_handling#workspace-error-handler'
	}}
	{textDisabled}
/>
