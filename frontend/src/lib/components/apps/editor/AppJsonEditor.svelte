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

	/**
	 * Open the JSON drawer for an item from the home list. `rawApp` MUST
	 * be set when the row is a raw-app draft, since `get_draft=true` has
	 * no deployed row to read the kind from — without it the backend
	 * looks for an `app` draft, doesn't find one, and 404s.
	 */
	export async function open(path_l: string, rawApp = false) {
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
			getDraft: true,
			rawApp
		})) as any
		app = { ...fapp }
		isDraftOnly = !!fapp.no_deployed
		isRawApp = !!fapp.raw_app || rawApp
		// Draft-only items: the editor's autosave writes the bare editable
		// shape (low-code: App; raw-app: `{files, runnables, data, ...}`)
		// straight into `draft`, so render that. The flattened `inner` has
		// the same content but is polluted with overlay fields
		// (`is_draft`, `no_deployed`, …) we don't want in the JSON.
		// Deployed items: the editable shape is the App definition under
		// `.value` (raw-app deploys keep the same response wrapper).
		const display = isDraftOnly ? fapp.draft : fapp.value
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
			// rename + deploy from the regular editor. `parsed` is already
			// the bare editable shape (App or raw-app value) — match what
			// the autosave writes so the regular editor reads it back
			// unchanged on next mount.
			await UserDraftDbSyncer.save({
				workspace: $workspaceStore!,
				itemKind: isRawApp ? 'raw_app' : 'app',
				path,
				value: parsed,
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
