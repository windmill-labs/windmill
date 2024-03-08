<script lang="ts">
	import { importStore } from '$lib/components/apps/store'

	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, Policy } from '$lib/gen'
	import { page } from '$app/stores'
	import { decodeState } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import type { App } from '$lib/components/apps/types'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import { DEFAULT_THEME } from '$lib/components/apps/editor/componentsPanel/themeUtils'

	let nodraft = $page.url.searchParams.get('nodraft')
	const hubId = $page.url.searchParams.get('hub')
	const templatePath = $page.url.searchParams.get('template')
	const templateId = $page.url.searchParams.get('template_id')

	const importRaw = $importStore
	if ($importStore) {
		$importStore = undefined
	}

	const state = nodraft ? undefined : localStorage.getItem('app')

	let summary = ''
	let value: App = {
		grid: [],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: [],
		theme: {
			type: 'path',
			path: DEFAULT_THEME
		}
	}

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	let policy: Policy = {
		on_behalf_of: $userStore?.username.includes('@')
			? $userStore?.username
			: `u/${$userStore?.username}`,
		on_behalf_of_email: $userStore?.email,
		execution_mode: Policy.execution_mode.PUBLISHER
	}

	loadApp()

	async function loadApp() {
		if (importRaw) {
			sendUserToast('Loaded from YAML/JSON')
			if ('value' in importRaw) {
				summary = importRaw.summary
				value = importRaw.value
				policy = importRaw.policy
			} else {
				value = importRaw
			}
		} else if (templatePath) {
			const template = await AppService.getAppByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			value = template.value
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (templateId) {
			const template = await AppService.getAppByVersion({
				workspace: $workspaceStore!,
				id: parseInt(templateId)
			})
			value = template.value
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (hubId) {
			const hub = await AppService.getHubAppById({ id: Number(hubId) })
			value = {
				hiddenInlineScripts: [],
				unusedInlineScripts: [],
				fullscreen: false,
				...hub.app.value
			}
			summary = hub.app.summary
			sendUserToast('App loaded from Hub')
			goto('?', { replaceState: true })
		} else if (!templatePath && !hubId && state) {
			sendUserToast('App restored from browser stored autosave', false, [
				{
					label: 'Start from blank',
					callback: () => {
						value = {
							grid: [],
							fullscreen: false,
							unusedInlineScripts: [],
							hiddenInlineScripts: [],
							theme: undefined
						}
					}
				}
			])
			value = decodeState(state)
		}
	}
</script>

{#if value}
	<div class="h-screen">
		{#key value}
			<AppEditor {summary} app={value} path={''} {policy} fromHub={hubId != null} />
		{/key}
	</div>
{/if}
