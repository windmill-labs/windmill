<script lang="ts">
	import { importStore } from '$lib/components/apps/store'

	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, type Policy } from '$lib/gen'
	import { page } from '$app/state'
	import { userStore, workspaceStore } from '$lib/stores'
	import type { App } from '$lib/components/apps/types'
	import { replaceState } from '$app/navigation'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'

	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import { DEFAULT_THEME } from '$lib/components/apps/editor/componentsPanel/themeUtils'
	import { emptyApp } from '$lib/components/apps/editor/appUtils'
	import { tick } from 'svelte'
	import { UserDraft } from '$lib/userDraft.svelte'

	// "+ App" buttons navigate with ?nodraft=true to signal "start fresh".
	// Wipe the persisted empty-path autosave and strip the flag from the URL
	// synchronously so a reload doesn't wipe the freshly-started draft. A
	// plain reload of /apps/add (no nodraft) instead restores the previous
	// session via the child AppEditor's `UserDraft.use`.
	if (page.url.searchParams.get('nodraft') && typeof window !== 'undefined') {
		UserDraft.remove('app', '')
		const url = new URL(window.location.href)
		url.searchParams.delete('nodraft')
		window.history.replaceState(window.history.state, '', url.toString())
	}
	let appEditor: AppEditor | undefined = $state(undefined)
	const hubId = page.url.searchParams.get('hub')
	const templatePath = page.url.searchParams.get('template')
	const templateId = page.url.searchParams.get('template_id')

	const importRaw = $importStore
	if ($importStore) {
		$importStore = undefined
	}

	let summary = $state('')
	let value: App = $state({
		grid: [],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: [],
		theme: {
			type: 'path',
			path: DEFAULT_THEME
		}
	})
	let policy: Policy = $state({
		on_behalf_of: $userStore?.username.includes('@')
			? $userStore?.username
			: `u/${$userStore?.username}`,
		on_behalf_of_email: $userStore?.email,
		execution_mode: 'publisher'
	})

	loadApp()

	async function loadApp() {
		if (importRaw) {
			// Import/template/hub loads are an explicit "start fresh from this
			// content" — drop any previous empty-path autosave so it doesn't
			// shadow the imported value on AppEditor mount.
			UserDraft.remove('app', '')
			sendUserToast('Loaded from YAML/JSON')
			if ('value' in importRaw) {
				summary = importRaw.summary
				value = importRaw.value
				policy = importRaw.policy
			} else {
				value = importRaw
			}
		} else if (templatePath) {
			UserDraft.remove('app', '')
			const template = await AppService.getAppByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			value = template.value as any
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (templateId) {
			UserDraft.remove('app', '')
			const template = await AppService.getAppByVersion({
				workspace: $workspaceStore!,
				id: parseInt(templateId)
			})
			value = template.value as any
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (hubId) {
			UserDraft.remove('app', '')
			const hub = await AppService.getHubAppById({ id: Number(hubId) })
			value = {
				hiddenInlineScripts: [],
				unusedInlineScripts: [],
				fullscreen: false,
				...((hub.app.value ?? {}) as any)
			}
			summary = hub.app.summary
			sendUserToast('App loaded from Hub')
			goto('?', { replaceState: true })
		} else {
			value = emptyApp()
		}

		// Trigger tutorial after everything is initialized
		const tutorialParam = page.url.searchParams.get('tutorial')
		if (tutorialParam) {
			// Wait for critical elements to be ready before triggering tutorial
			await tick()
			let attempts = 0
			while (attempts < 20 && !document.querySelector('#app-editor-runnable-panel')) {
				await new Promise((resolve) => setTimeout(resolve, 100))
				attempts++
			}
			appEditor?.triggerTutorial()
		}
	}
</script>

{#if value}
	<div class="h-screen">
		{#key value}
			<AppEditor
				bind:this={appEditor}
				onSavedNewAppPath={(path) => {
					goto(`/apps/edit/${path}`)
				}}
				{summary}
				app={value}
				path={''}
				{policy}
				fromHub={hubId != null}
				newApp={true}
				replaceStateFn={(path) => replaceState(path, page.state)}
				gotoFn={(path, opt) => goto(path, opt)}
			>
				{#snippet unsavedConfirmationModal({
					diffDrawer,
					additionalExitAction,
					getInitialAndModifiedValues
				})}
					<UnsavedConfirmationModal
						{diffDrawer}
						{additionalExitAction}
						{getInitialAndModifiedValues}
					/>
				{/snippet}
			</AppEditor>
		{/key}
	</div>
{/if}
