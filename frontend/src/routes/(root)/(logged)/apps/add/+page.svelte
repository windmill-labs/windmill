<script lang="ts">
	import { importStore } from '$lib/components/apps/store'

	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, type Policy } from '$lib/gen'
	import { page } from '$app/stores'
	import { decodeState } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import type { App } from '$lib/components/apps/types'
	import { afterNavigate, replaceState } from '$app/navigation'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'

	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import { DEFAULT_THEME } from '$lib/components/apps/editor/componentsPanel/themeUtils'
	import {
		presets,
		processDimension,
		type AppComponent
	} from '$lib/components/apps/editor/component'
	import {
		appComponentFromType,
		insertNewGridItem,
		setUpTopBarComponentContent
	} from '$lib/components/apps/editor/appUtils'
	import { usePromise } from '$lib/svelte5Utils.svelte'

	let nodraft = $page.url.searchParams.get('nodraft')
	const hubId = $page.url.searchParams.get('hub')
	const templatePath = $page.url.searchParams.get('template')
	const templateId = $page.url.searchParams.get('template_id')

	const importRaw = $importStore
	if ($importStore) {
		$importStore = undefined
	}

	const appState = nodraft ? undefined : localStorage.getItem('app')

	let data = usePromise(loadApp)

	afterNavigate(() => {
		if (nodraft) {
			let url = new URL($page.url.href)
			url.search = ''
			replaceState(url.toString(), $page.state)
		}
	})
	let policy: Policy = $state({
		on_behalf_of: $userStore?.username.includes('@')
			? $userStore?.username
			: `u/${$userStore?.username}`,
		on_behalf_of_email: $userStore?.email,
		execution_mode: 'publisher'
	})

	async function loadApp() {
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
			value = template.value as any
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (templateId) {
			const template = await AppService.getAppByVersion({
				workspace: $workspaceStore!,
				id: parseInt(templateId)
			})
			value = template.value as any
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (hubId) {
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
		} else if (!templatePath && !hubId && appState) {
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
			value = decodeState(appState)
		} else {
			const preset = presets['topbarcomponent']

			const id = insertNewGridItem(
				value,
				appComponentFromType(preset.targetComponent, preset.configuration, undefined, {
					customCss: {
						container: {
							class: '!p-0' as any,
							style: ''
						}
					}
				}) as (id: string) => AppComponent,
				undefined,
				undefined,
				'topbar',
				{ x: 0, y: 0 },
				{
					3: processDimension(preset.dims, 3),
					12: processDimension(preset.dims, 12)
				},
				true,
				true
			)

			setUpTopBarComponentContent(id, value)

			value.hideLegacyTopBar = true
			value.mobileViewOnSmallerScreens = false
		}
		return { app: value, summary }
	}
</script>

{#if data.status === 'ok'}
	{@const { app, summary } = data.value}
	<div class="h-screen">
		<AppEditor
			on:savedNewAppPath={(event) => {
				goto(`/apps/edit/${event.detail}`)
			}}
			{summary}
			appInitial={app}
			path={''}
			{policy}
			fromHub={hubId != null}
			newApp={true}
			replaceStateFn={(path) => replaceState(path, $page.state)}
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
	</div>
{/if}
