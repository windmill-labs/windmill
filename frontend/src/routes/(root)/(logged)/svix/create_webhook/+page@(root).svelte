<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button } from '$lib/components/common'
	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	let scriptPath: string | undefined = undefined
	let itemKind: 'script' | 'flow' | 'app' = 'script'
	let clientName = $page.url.searchParams.get('name') ?? 'Svix'
	let redirectURI = $page.url.searchParams.get('redirect_uri')

	async function createWebhook(path: string, kind: 'script' | 'flow' | 'app') {
		try {
			let webhook
			switch (kind) {
				case 'script':
					webhook = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/p/${path}`
					redirectSuccess(webhook)
					break
				case 'flow':
					webhook = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/f/${path}`
					redirectSuccess(webhook)
					break
				default:
					sendUserToast('Invalid item type', true)
					break
			}
		} catch (error) {
			sendUserToast(error, true)
		}
	}

	function redirectSuccess(webhook: string) {
		const url = new URL(redirectURI!)
		url.searchParams.append('webhook_url', webhook)
		window.location.href = url.toString()
	}

	function redirectCancel(e: string): void {
		const url = new URL(redirectURI!)
		url.searchParams.append('error', e)
		window.location.href = url.toString()
	}
</script>

{#if redirectURI != null}
	<CenteredPage>
		<div class="flex items-center justify-center h-screen">
			<div class="flex flex-col items-center gap-5">
				<PageHeader title={`${clientName} wants to create a webhook`} />
				<h2>Select the script or flow to be triggered by the webhook</h2>
				<ScriptPicker
					allowEdit={false}
					allowFlow={true}
					allowRefresh={true}
					bind:scriptPath
					bind:itemKind
				/>
				<div class="flex flex-row gap-2">
					<Button
						disabled={scriptPath == undefined}
						on:click={() => scriptPath && createWebhook(scriptPath, itemKind)}>Select script</Button
					>
					<Button on:click={() => redirectCancel('user_canceled')}>Cancel</Button>
				</div>
				<h2>or</h2>
				<div class="flex flex-row gap-2">
					<Button>Create New Script</Button>
					<Button>Create New Flow</Button>
				</div>
			</div>
		</div>
	</CenteredPage>
{/if}
