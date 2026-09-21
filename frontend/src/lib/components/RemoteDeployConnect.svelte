<script lang="ts">
	import { WorkspaceService, type RemoteDeployConnection, type RemoteDeployTarget } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { ExternalLink, Unplug } from 'lucide-svelte'
	import { Button } from './common'
	import Password from './Password.svelte'

	let {
		workspace,
		target,
		connection,
		onChange
	}: {
		workspace: string
		target: RemoteDeployTarget
		connection: RemoteDeployConnection | undefined
		onChange: () => void
	} = $props()

	let token: string | undefined = $state(undefined)
	let connecting = $state(false)
	let error: string | undefined = $state(undefined)

	async function connect() {
		if (!token) return
		connecting = true
		error = undefined
		try {
			await WorkspaceService.connectRemoteDeploy({ workspace, requestBody: { token } })
			token = undefined
			onChange()
		} catch (e: any) {
			error = e?.body ?? e?.message ?? String(e)
		} finally {
			connecting = false
		}
	}

	async function disconnect() {
		await WorkspaceService.disconnectRemoteDeploy({ workspace })
		sendUserToast(`Disconnected from ${target.base_url}`)
		onChange()
	}
</script>

{#if connection}
	<div class="flex flex-row items-center gap-2 text-xs text-secondary">
		<span>
			Deploying as <span class="font-semibold text-primary">{connection.remote_email}</span> on
			{target.base_url}
		</span>
		<Button variant="subtle" unifiedSize="xs" startIcon={{ icon: Unplug }} onclick={disconnect}>
			Disconnect
		</Button>
	</div>
{:else}
	<div class="flex flex-col gap-2 max-w-xl">
		<p class="text-xs text-secondary">
			Deploys to {target.workspace_id} on {target.base_url} run with your own account on that instance,
			so its permissions, protection rules and audit logs apply. Create a token there and paste it below.
			It is stored encrypted and only ever sent to that instance.
		</p>
		<a
			class="text-xs flex flex-row items-center gap-1 w-fit"
			href="{target.base_url}/#user-settings"
			target="_blank"
			rel="noopener noreferrer"
		>
			Create a token on {target.base_url}
			<ExternalLink size={12} />
		</a>
		<div class="flex flex-row gap-2 items-start">
			<div class="grow">
				<Password
					bind:password={token}
					small
					allowMultiline={false}
					placeholder="Token for {target.base_url}"
					error={error != undefined}
					onKeyDown={(e) => e.key == 'Enter' && connect()}
				/>
			</div>
			<Button
				variant="accent"
				unifiedSize="sm"
				disabled={!token || connecting}
				loading={connecting}
				onclick={connect}
			>
				Connect
			</Button>
		</div>
		{#if error}
			<p class="text-xs text-red-500 dark:text-red-400 break-words">{error}</p>
		{/if}
	</div>
{/if}
