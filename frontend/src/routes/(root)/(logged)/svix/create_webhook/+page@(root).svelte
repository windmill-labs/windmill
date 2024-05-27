<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button } from '$lib/components/common'
	import { page } from '$app/stores'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import { generateRandomString } from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'

	let itemPath: string | undefined = undefined
	let itemKind: 'script' | 'flow' | 'app' = 'script'
	let clientName = $page.url.searchParams.get('name') ?? undefined
	let redirectURI = $page.url.searchParams.get('redirect_uri')

	let userSettings: UserSettings
	let token: string | undefined = undefined
	let scopes: string[] = []

	$: updateTokenAndScope(itemPath)

	async function createWebhook(path: string, kind: 'script' | 'flow' | 'app', token: string) {
		try {
			let webhook: string
			switch (kind) {
				case 'script':
					webhook = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/p/${path}`
					redirectSuccess(webhook, token)
					break
				case 'flow':
					webhook = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/f/${path}`
					redirectSuccess(webhook, token)
					break
				default:
					sendUserToast('Invalid item type', true)
					break
			}
		} catch (error) {
			sendUserToast(error, true)
		}
	}

	function redirectSuccess(webhook: string, token: string) {
		const webhook_url = new URL(webhook)
		webhook_url.searchParams.append('token', token)
		const url = new URL(redirectURI!)
		url.searchParams.append('webhook_url', webhook_url.toString())
		window.location.href = url.toString()
	}

	function redirectCancel(e: string): void {
		const url = new URL(redirectURI!)
		url.searchParams.append('error', e)
		window.location.href = url.toString()
	}

	function updateTokenAndScope(scriptPath: string | undefined) {
		if (!scriptPath) {
			scopes = []
			token = undefined
			return
		}

		token = undefined
		scopes = [`run:${itemKind}/${scriptPath}`]
	}
</script>

<UserSettings
	bind:this={userSettings}
	on:tokenCreated={(e) => {
		token = e.detail
	}}
	newTokenLabel={`webhook-${$userStore?.username ?? 'superadmin'}-${generateRandomString(4)}`}
	{scopes}
/>

<CenteredPage>
	<div class="flex items-center justify-center h-screen">
		<div class="flex flex-col gap-5">
			<div class="flex flex-row gap-3 items-center justify-center">
				<PageHeader title={`${clientName} wants to create a webhook`} />
			</div>
			<div class="flex flex-row gap-3 items-center">
				<div
					class="flex items-center justify-center w-10 h-10 border-2 border-blue-500 text-blue-500 font-bold rounded-full"
				>
					1
				</div>
				<h2>Select the script or flow to be triggered by the webhook</h2>
			</div>
			<div class="flex flex-row items-center justify-center w-100%">
				<div class="flex flex-col items-center gap-3 w-full">
					<ScriptPicker
						allowEdit={false}
						allowFlow={true}
						allowRefresh={true}
						bind:scriptPath={itemPath}
						bind:itemKind
					/>
					<h4>or</h4>
					<div class="flex flex-row gap-2">
						<Button size="xs" color="light" variant="border" target="_blank" href="/scripts/add">Create New Script</Button>
						<Button size="xs" color="light" variant="border" target="_blank" href="/flows/add?nodraft=true">Create New Flow</Button>
					</div>
				</div>
			</div>
			<div class="flex flex-row gap-3 items-center mt-3">
				<div
					class="flex items-center justify-center w-10 h-10 border-2 border-blue-500 text-blue-500 font-bold rounded-full"
				>
					2
				</div>
				<h2>Create a token for this webhook</h2>
				<Tooltip customSize="150%">
					If you generate a token, it will have a scope such that it can only be used to
					trigger this {itemKind}. It is safe to share as it cannot be used to impersonate you.
					Otherwise you may enter another token but beware that it will be shared with the external
					application.
				</Tooltip>
			</div>
			<div class="flex flex-row items-center justify-center">
				<div class="flex flex-col items-center gap-5 w-100%">
					<div class="flex flex-row items-center gap-2">
						<input
							bind:value={token}
							disabled={itemPath == undefined}
							placeholder="enter or generate token"
							class="!text-md"
						/>
						<Button size="md" disabled={itemPath == undefined} on:click={userSettings.openDrawer}>
							Generate webhook-specific Token
						</Button>
					</div>
					<div class="flex flex-row gap-2 mt-10">
						<Button
							disabled={!itemPath || !token || token === ''}
							on:click={() => itemPath && token && createWebhook(itemPath, itemKind, token)}
							>Approve and share with {clientName}</Button
						>
						<Button color="red" on:click={() => redirectCancel('user_canceled')}>Cancel</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
</CenteredPage>
