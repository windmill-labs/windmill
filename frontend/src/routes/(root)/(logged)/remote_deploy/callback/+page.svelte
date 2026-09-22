<script lang="ts">
	import { goto, replaceState } from '$app/navigation'
	import { page } from '$app/state'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Alert, Button } from '$lib/components/common'
	import { WorkspaceService } from '$lib/gen'
	import { CHANNEL, takePendingConnect, type RemoteDeployEvent } from '$lib/remoteDeploy'
	import { onMount } from 'svelte'
	import { Loader2 } from 'lucide-svelte'

	// Where the remote's authorize page sends the token it minted, in the fragment so that no server
	// log or Referer ever holds it.
	const fragment = new URLSearchParams(window.location.hash.slice(1))

	let status: 'connecting' | 'connected' | 'failed' = $state('connecting')
	let error: string | undefined = $state(undefined)
	let returnTo: string | undefined = $state(undefined)

	function announce(event: RemoteDeployEvent) {
		const channel = new BroadcastChannel(CHANNEL)
		channel.postMessage(event)
		channel.close()
	}

	onMount(async () => {
		// Nor the address bar and the history, once read.
		replaceState(page.url.pathname, page.state)
		const pending = takePendingConnect(fragment.get('state') ?? '')
		if (!pending) {
			status = 'failed'
			error =
				'This connection was not started from this browser, or took too long. Start again from the deploy drawer.'
			return
		}
		returnTo = pending.returnTo
		const token = fragment.get('token')
		try {
			if (!token) {
				throw new Error(fragment.get('error') ?? 'The remote instance sent no token')
			}
			await WorkspaceService.connectRemoteDeploy({
				workspace: pending.workspace,
				requestBody: { token, target: pending.target }
			})
		} catch (e: any) {
			const message: string = e?.body ?? e?.message ?? String(e)
			status = 'failed'
			error = message
			announce({ type: 'failed', workspace: pending.workspace, error: message })
			return
		}
		status = 'connected'
		announce({ type: 'connected', workspace: pending.workspace })
		// Only a window this instance opened can close itself; otherwise go back to where the
		// connection was started.
		window.close()
		await goto(pending.returnTo)
	})
</script>

<CenteredModal title="Connecting to the remote instance">
	{#if status === 'connecting'}
		<div class="flex justify-center py-6"><Loader2 class="animate-spin" /></div>
	{:else if status === 'connected'}
		<p class="text-sm text-primary text-center">Connected. You can close this tab.</p>
	{:else}
		<Alert type="error" title="Could not connect">{error}</Alert>
		{#if returnTo}
			<div class="flex justify-center pt-4">
				<Button variant="default" unifiedSize="md" onclick={() => goto(returnTo!)}>Go back</Button>
			</div>
		{/if}
	{/if}
</CenteredModal>
