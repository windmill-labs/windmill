<script lang="ts">
	import { importStore } from '$lib/components/apps/store'

	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, Policy } from '$lib/gen'
	import { page } from '$app/stores'
	import { decodeState, sendUserToast } from '$lib/utils'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { userStore, workspaceStore } from '$lib/stores'
	import type { App } from '$lib/components/apps/types'
	import { goto } from '$app/navigation'

	let nodraft = $page.url.searchParams.get('nodraft')
	const hubId = $page.url.searchParams.get('hub')
	const templatePath = $page.url.searchParams.get('template')

	const importJson = $importStore
	if ($importStore) {
		$importStore = undefined
	}

	const state = nodraft ? undefined : localStorage.getItem('app')

	let summary = ''
	let value: App = {
		grid: [],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: []
	}

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	loadApp()

	async function loadApp() {
		if (importJson) {
			sendUserToast('Loaded from raw JSON')
			value = importJson
		} else if (templatePath) {
			const template = await AppService.getAppByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			value = template.value
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (hubId) {
			const hub = await AppService.getHubAppById({ id: Number(hubId) })
			value = hub.app.value
			summary = hub.app.summary
			sendUserToast('App loaded from Hub')
			goto('?', { replaceState: true })
		} else if (!templatePath && !hubId && state) {
			sendUserToast('App restored from draft')
			value = decodeState(state)
		}
	}

	$dirtyStore = false
</script>

{#if value}
	<div class="h-screen">
		{#key value}
			<AppEditor
				{summary}
				app={value}
				path={''}
				policy={{
					on_behalf_of: `u/${$userStore?.username}`,
					on_behalf_of_email: $userStore?.email,
					execution_mode: Policy.execution_mode.PUBLISHER
				}}
			/>
		{/key}
	</div>
{/if}
