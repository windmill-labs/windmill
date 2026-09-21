<script lang="ts">
	import { page } from '$app/state'
	import { WorkspaceService, type RemoteDeployConnection, type RemoteDeployTarget } from '$lib/gen'
	import {
		CHANNEL,
		remoteDeployAuthorizeUrl,
		remoteHost,
		type RemoteDeployEvent
	} from '$lib/remoteDeploy'
	import { sendUserToast } from '$lib/toast'
	import { ExternalLink, LogIn, Unplug } from 'lucide-svelte'
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

	let host = $derived(remoteHost(target.base_url))
	let token: string | undefined = $state(undefined)
	let connecting = $state(false)
	let waitingForRemote = $state(false)
	let showPaste = $state(false)
	let error: string | undefined = $state(undefined)

	function signIn() {
		error = undefined
		const url = remoteDeployAuthorizeUrl(target, workspace, page.url.pathname + page.url.search)
		// Opened blank and cut loose before it navigates: the remote page gets no handle on this
		// window, and a blocked popup is still detected.
		const popup = window.open('', 'windmill-remote-deploy', 'popup,width=640,height=760')
		if (!popup) {
			window.location.href = url
			return
		}
		popup.opener = null
		popup.location.href = url
		waitingForRemote = true
		const watch = setInterval(() => {
			if (popup.closed) {
				clearInterval(watch)
				waitingForRemote = false
			}
		}, 500)
	}

	// The callback page runs in the popup, or in this tab when the popup was blocked.
	$effect(() => {
		const channel = new BroadcastChannel(CHANNEL)
		channel.onmessage = (event: MessageEvent<RemoteDeployEvent>) => {
			if (event.data?.workspace !== workspace) return
			waitingForRemote = false
			if (event.data.type === 'connected') {
				onChange()
			} else {
				error = event.data.error
			}
		}
		return () => channel.close()
	})

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
	<div class="flex flex-col gap-3 max-w-xl">
		<p class="text-xs text-secondary">
			Deploys to {target.workspace_id} on {target.base_url} run with your own account on that instance,
			so its permissions, protection rules and audit logs apply. Sign in there to connect; the token
			it issues is stored encrypted and only ever sent to that instance.
		</p>
		<div class="flex flex-row items-center gap-3">
			<Button variant="accent" unifiedSize="sm" startIcon={{ icon: LogIn }} onclick={signIn}>
				Sign in to {host}
			</Button>
			{#if waitingForRemote}
				<span class="text-xs text-secondary">Waiting for {host}…</span>
			{/if}
			<Button variant="subtle" unifiedSize="sm" onclick={() => (showPaste = !showPaste)}>
				{showPaste ? 'Hide token field' : 'Paste a token instead'}
			</Button>
		</div>
		{#if showPaste}
			<div class="flex flex-col gap-2">
				<a
					class="text-xs flex flex-row items-center gap-1 w-fit"
					href="{target.base_url}/#user-settings"
					target="_blank"
					rel="noopener noreferrer"
				>
					Create a token on {host}
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
						variant="default"
						unifiedSize="sm"
						disabled={!token || connecting}
						loading={connecting}
						onclick={connect}
					>
						Connect
					</Button>
				</div>
			</div>
		{/if}
		{#if error}
			<p class="text-xs text-red-500 dark:text-red-400 break-words">{error}</p>
		{/if}
	</div>
{/if}
