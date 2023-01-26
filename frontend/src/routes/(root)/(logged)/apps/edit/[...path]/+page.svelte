<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { page } from '$app/stores'
	import { decodeState, sendUserToast } from '$lib/utils'
	import { goto } from '$app/navigation'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	let app: AppWithLastVersion | undefined = undefined
	let path = $page.params.path

	let nodraft = $page.url.searchParams.get('nodraft')

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	let loading = true

	async function loadApp(): Promise<void> {
		loading = true
		app = await AppService.getAppByPath({
			path,
			workspace: $workspaceStore!
		})

		const initialState = nodraft ? undefined : localStorage.getItem(`app-${$page.params.path}`)
		let stateLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined
		if (stateLoadedFromUrl) {
			sendUserToast('App restored from draft')
			app.value = stateLoadedFromUrl
		}
		loading = false
		$dirtyStore = false
	}

	$: {
		if ($workspaceStore) {
			loadApp()
		}
	}
</script>

{#if app}
	<div class="h-screen">
		<AppEditor summary={app.summary} app={app.value} path={app.path} policy={app.policy} />
	</div>
{/if}
