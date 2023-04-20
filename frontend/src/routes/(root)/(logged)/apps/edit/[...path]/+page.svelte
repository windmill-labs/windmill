<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { page } from '$app/stores'
	import { decodeState, sendUserToast, type ToastAction } from '$lib/utils'
	import { goto } from '$app/navigation'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	$: app = undefined as AppWithLastVersion | undefined

	let redraw = 0
	let path = $page.params.path

	let nodraft = $page.url.searchParams.get('nodraft')

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	function restoreTempValue(tempValue) {
		if (app) {
			app.value = tempValue
			app = app
			redraw++
		}
	}

	async function loadApp(): Promise<void> {
		app = await AppService.getAppByPath({
			path,
			workspace: $workspaceStore!
		})

		const tempValue = app.value

		const initialState = nodraft ? undefined : localStorage.getItem(`app-${$page.params.path}`)
		let stateLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined

		if (stateLoadedFromUrl) {
			const actions: ToastAction[] = []
			if (JSON.stringify(app.value) !== JSON.stringify(stateLoadedFromUrl)) {
				actions.push({
					label: 'Load from last save instead',
					callback: () => {
						restoreTempValue(tempValue)
					}
				})
			}

			sendUserToast('App restored from draft', false, actions)
			app.value = stateLoadedFromUrl
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
