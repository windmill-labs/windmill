<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { ScriptService, FlowService } from '$lib/gen'
	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'
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
			switch (kind) {
				case 'script':
					const webhook = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/p/${path}`
					redirectSuccess(webhook)
					break
				case 'flow':
					const webhook = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/f/${path}`
					redirectSuccess(webhook)
					break
				default:
					sendUserToast("Invalid item type", true)
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

{#if redirectURI == undefined}
	<CenteredPage>404</CenteredPage>
{:else}
	<CenteredPage>
		<div class="flex items-center justify-center h-screen">
			<div class="flex flex-col items-center gap-5">
				{#if clientName != undefined}
					<PageHeader title={`${clientName} wants to create a webhook`} />
				{:else}
					<PageHeader title="Svix wants to create a webhook" />
				{/if}
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
						disabled={scriptPath === undefined}
						on:click={() => createWebhook(scriptPath, itemKind)}>Select script</Button
					>
					<Button on:click={() => redirectCancel("user_canceled")}>Cancel</Button>
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
