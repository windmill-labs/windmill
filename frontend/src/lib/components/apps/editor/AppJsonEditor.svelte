<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'

	import JsonEditor from '../../JsonEditor.svelte'
	import { AppService } from '$lib/gen'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { sendUserToast } from '$lib/toast'
	import { userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { Globe, Loader2, Save } from 'lucide-svelte'

	let jsonViewerDrawer: Drawer | undefined = $state()

	let code: string = $state('')
	let path: string = ''
	let loading = $state(true)
	let isDraftOnly = $state(false)
	let isRawApp = $state(false)
	const dispatch = createEventDispatcher()

	let app: any | undefined = undefined

	export async function open(path_l: string) {
		loading = true
		jsonViewerDrawer?.toggleDrawer()
		path = path_l
		// `get_draft=true` so draft-only items at `u/{user}/draft_{uuid}`
		// resolve to the synthesized draft stand-in instead of 404'ing the
		// home-page "View/Edit JSON" menu entry. Deployed apps continue to
		// return the deployed payload unchanged (see WithDraftOverlay).
		const fapp = (await AppService.getAppByPath({
			workspace: $workspaceStore!,
			path,
			getDraft: true
		})) as any
		app = { ...fapp }
		isDraftOnly = !!fapp.no_deployed
		isRawApp = !!fapp.raw_app
		// For deployed (or app drafts overlaid on a deployed row) the
		// editable shape is the App definition under `.value`. For raw-app
		// drafts there is no nested `.value` — the editable shape is the
		// whole flattened draft (`{files, runnables, data, ...}`).
		const display = isDraftOnly && isRawApp ? fapp : fapp.value
		code = JSON.stringify(display, null, 4)
		loading = false
	}

	export async function saveApp() {
		const parsed = JSON.parse(code)
		if (isDraftOnly) {
			// Draft-only items have no deployed row — `updateApp` would
			// 404. Route the edit through the syncer (`immediate: true`
			// so the caller's `await` resolves only after the POST lands)
			// so the user keeps editing under the draft path until they
			// rename + deploy from the regular editor.
			const value = isRawApp ? { ...app, ...parsed } : { ...app, value: parsed }
			await UserDraftDbSyncer.save({
				workspace: $workspaceStore!,
				itemKind: isRawApp ? 'raw_app' : 'app',
				path,
				value,
				immediate: true
			})
			dispatch('change')
			sendUserToast('Draft saved')
		} else {
			await AppService.updateApp({
				workspace: $workspaceStore!,
				path,
				requestBody: { ...app, value: parsed }
			})
			dispatch('change')
			UserDraft.remove('app', path)
			sendUserToast('App deployed')
		}
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
				<Button
					on:click={saveApp}
					startIcon={{ icon: isDraftOnly ? Save : Globe }}
					variant="accent"
					size="xs"
				>
					{isDraftOnly ? 'Save draft' : 'Deploy'}
				</Button>
			{/if}
		{/snippet}
	</DrawerContent>
</Drawer>
