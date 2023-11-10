<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'

	import JsonEditor from './settingsPanel/inputEditor/JsonEditor.svelte'
	import { AppService, DraftService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { Loader2, Save } from 'lucide-svelte'

	let jsonViewerDrawer: Drawer

	let code: string = ''
	let path: string = ''
	let useDraft: boolean = false
	let loading = true
	const dispatch = createEventDispatcher()

	let app: any | undefined = undefined

	export async function open(path_l: string) {
		loading = true
		jsonViewerDrawer?.toggleDrawer()
		path = path_l
		const fapp = await AppService.getAppByPathWithDraft({
			workspace: $workspaceStore!,
			path
		})
		useDraft = fapp?.draft != undefined
		app = { ...fapp }
		if (fapp.draft) {
			delete app['draft']
		}
		const capp = fapp?.draft ? fapp.draft : fapp.value
		code = JSON.stringify(capp, null, 4)
		loading = false
	}

	export async function saveApp() {
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path,
			requestBody: { ...app, value: JSON.parse(code) }
		})
		dispatch('change')
		localStorage.removeItem(`app-${path}`)
		sendUserToast('App deployed')
	}

	export async function saveDraft() {
		await DraftService.createDraft({
			workspace: $workspaceStore!,
			requestBody: {
				path: path,
				typ: 'app',
				value: JSON.parse(code)
			}
		})
		dispatch('change')
		localStorage.removeItem(`app-${path}`)

		sendUserToast('Draft saved')
	}
</script>

<Drawer bind:this={jsonViewerDrawer} size="800px">
	<DrawerContent title="App JSON" on:close={() => jsonViewerDrawer.toggleDrawer()}>
		{#if useDraft}
			<div class="mb-1">
				<Badge small color="indigo" baseClass="border border-indigo-200">+Draft</Badge>
			</div>
		{/if}
		{#if loading}
			<Loader2 class="animate-spin" />
		{:else}
			<JsonEditor bind:code />
		{/if}

		<svelte:fragment slot="actions">
			<Button on:click={saveDraft} startIcon={{ icon: Save }}>Save as draft</Button>

			<Button on:click={saveApp} startIcon={{ icon: Save }}>Deploy</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
