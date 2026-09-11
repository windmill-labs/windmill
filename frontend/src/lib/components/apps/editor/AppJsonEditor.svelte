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
	import { deployRawAppValue } from '$lib/rawAppDeploy'

	let jsonViewerDrawer: Drawer | undefined = $state()

	let code: string = $state('')
	let path: string = ''
	let loading = $state(true)
	let isDraftOnly = $state(false)
	let isRawApp = $state(false)
	const dispatch = createEventDispatcher()

	let app: any | undefined = undefined

	/**
	 * Open the JSON drawer for a home-list item. `rawApp` MUST be set for a
	 * raw-app draft, since a draft-only row has no deployed kind to read and
	 * the backend would 404 looking for an `app` draft.
	 */
	export async function open(path_l: string, rawApp = false) {
		loading = true
		jsonViewerDrawer?.toggleDrawer()
		path = path_l
		// `get_draft=true` so draft-only items resolve to the synthesized draft
		// stand-in instead of 404'ing; deployed apps return unchanged.
		const fapp = (await AppService.getAppByPath({
			workspace: $workspaceStore!,
			path,
			getDraft: true,
			rawApp
		})) as any
		app = { ...fapp }
		isDraftOnly = !!fapp.no_deployed
		isRawApp = !!fapp.raw_app || rawApp
		// Draft-only: render `draft` (the bare editable shape) rather than the
		// overlay-polluted top level. Deployed: the editable shape is `.value`.
		const display = isDraftOnly ? fapp.draft : fapp.value
		code = JSON.stringify(display, null, 4)
		loading = false
	}

	// The button drops a rejection, so failures — invalid JSON, a raw app whose
	// sources don't bundle — have to surface here or Deploy looks like a no-op.
	export async function saveApp() {
		try {
			await deploy()
		} catch (err: any) {
			sendUserToast(`Could not save app: ${err.body ?? err.message}`, true)
		}
	}

	async function deploy() {
		const parsed = JSON.parse(code)
		if (isDraftOnly) {
			// No deployed row — `updateApp` would 404. Route through the syncer
			// (`immediate: true` so `await` resolves after the POST). `parsed` is
			// the bare editable shape the autosave writes, so it reads back unchanged.
			await UserDraftDbSyncer.save({
				workspace: $workspaceStore!,
				itemKind: isRawApp ? 'raw_app' : 'app',
				path,
				value: parsed,
				immediate: true
			})
			dispatch('change')
			sendUserToast('Draft saved')
		} else if (isRawApp) {
			// A raw app's value holds its sources, so deploying it means re-bundling.
			await deployRawAppValue({
				workspace: $workspaceStore!,
				path,
				value: parsed,
				summary: app?.summary,
				policy: app?.policy,
				customPath: app?.custom_path
			})
			dispatch('change')
			UserDraft.remove('raw_app', path)
			sendUserToast('App deployed')
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
