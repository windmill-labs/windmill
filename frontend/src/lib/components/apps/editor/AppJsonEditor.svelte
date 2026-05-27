<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'

	import JsonEditor from '../../JsonEditor.svelte'
	import { AppService } from '$lib/gen'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { sendUserToast } from '$lib/toast'
	import { userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { Globe, Loader2 } from 'lucide-svelte'

	let jsonViewerDrawer: Drawer | undefined = $state()

	let code: string = $state('')
	let path: string = ''
	let loading = $state(true)
	const dispatch = createEventDispatcher()

	let app: any | undefined = undefined

	export async function open(path_l: string) {
		loading = true
		jsonViewerDrawer?.toggleDrawer()
		path = path_l
		const fapp = await AppService.getAppByPath({
			workspace: $workspaceStore!,
			path
		})
		app = { ...fapp }
		code = JSON.stringify(fapp.value, null, 4)
		loading = false
	}

	export async function saveApp() {
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path,
			requestBody: { ...app, value: JSON.parse(code) }
		})
		dispatch('change')
		UserDraft.remove('app', path)
		sendUserToast('App deployed')
	}
</script>

<Drawer bind:this={jsonViewerDrawer} size="800px">
	<DrawerContent title="App JSON" on:close={() => jsonViewerDrawer?.toggleDrawer()}>
		{#if loading}
			<Loader2 class="animate-spin" />
		{:else}
			<JsonEditor bind:code />
		{/if}

		{#snippet actions()}
			{#if !$userStore?.operator}
				<Button on:click={saveApp} startIcon={{ icon: Globe }} variant="accent" size="xs">
					Deploy
				</Button>
			{/if}
		{/snippet}
	</DrawerContent>
</Drawer>
