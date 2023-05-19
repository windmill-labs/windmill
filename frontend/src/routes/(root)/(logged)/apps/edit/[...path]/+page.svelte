<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { page } from '$app/stores'
	import { decodeState } from '$lib/utils'
	import { goto } from '$app/navigation'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { sendUserToast, type ToastAction } from '$lib/toast'

	let app = undefined as (AppWithLastVersion & { draft_only?: boolean }) | undefined

	let redraw = 0
	let path = $page.params.path

	let nodraft = $page.url.searchParams.get('nodraft')

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	const initialState = nodraft ? undefined : localStorage.getItem(`app-${$page.params.path}`)
	let stateLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined

	async function loadApp(): Promise<void> {
		const app_w_draft = await AppService.getAppByPathWithDraft({
			path,
			workspace: $workspaceStore!
		})

		if (stateLoadedFromUrl) {
			const actions: ToastAction[] = []
			if (stateLoadedFromUrl) {
				actions.push({
					label: 'Discard autosave and reload',
					callback: async () => {
						stateLoadedFromUrl = undefined
						await loadApp()
						redraw++
					}
				})
			}

			sendUserToast('App restored from ephemeral autosave', false, actions)
			app_w_draft.value = stateLoadedFromUrl
			app = app_w_draft
		} else if (app_w_draft.draft) {
			app = {
				...app_w_draft,
				value: app_w_draft.draft
			}
			if (!app_w_draft.draft_only) {
				sendUserToast('flow loaded from latest saved draft', false, [
					{
						label: 'Ignore draft and load from latest deployed version',
						callback: () => {
							stateLoadedFromUrl = undefined
							app = app_w_draft
							redraw++
						}
					}
				])
			}
		} else {
			app = app_w_draft
		}
		$dirtyStore = false
	}

	$: {
		if ($workspaceStore) {
			loadApp()
		}
	}
</script>

{#key redraw}
	{#if app}
		<div class="h-screen">
			<AppEditor summary={app.summary} app={app.value} path={app.path} policy={app.policy} />
		</div>
	{/if}
{/key}
