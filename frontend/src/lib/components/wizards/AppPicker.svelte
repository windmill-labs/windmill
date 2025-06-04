<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import SelectLegacy from '../apps/svelte-select/lib/SelectLegacy.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { AppService, type ListableApp } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import type { AppViewerContext } from '../apps/types'
	import Alert from '../common/alert/Alert.svelte'

	export let value = ''
	export let selecteValue = value
	let darkMode = false

	const { appPath } = getContext<AppViewerContext>('AppViewerContext')

	let apps: ListableApp[] = []

	async function loadApps(): Promise<void> {
		apps = (await AppService.listApps({ workspace: $workspaceStore!, includeDraftOnly: true })).map(
			(app: ListableApp) => {
				return {
					canWrite:
						canWrite(app.path!, app.extra_perms!, $userStore) &&
						app.workspace_id == $workspaceStore &&
						!$userStore?.operator,
					...app
				}
			}
		)
	}

	onMount(() => {
		loadApps()

		if (selecteValue === '') {
			selecteValue = $appPath
			value = $appPath
		}
	})
</script>

<DarkModeObserver bind:darkMode />

<div class="flex flex-col gap-2 w-full">
	<SelectLegacy
		class="grow shrink max-w-full"
		on:change={(e) => {
			value = e.detail.value
		}}
		on:clear={() => {
			value = ''
		}}
		bind:value={selecteValue}
		items={apps.map((app) => {
			return {
				value: app.path,
				label: app.path === $appPath ? `${app.path} (current app)` : app.path
			}
		})}
		placeholder="Pick an app"
		inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
		containerStyles={darkMode
			? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
			: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
		portal={false}
	/>
	{#if !appPath}
		<Alert title="Current app not selectable" size="xs" type="warning" collapsible>
			Current app is not selectable until you have deployed this app at least once.
		</Alert>
	{/if}
	{#if appPath && $appPath === value}
		<div class="text-2xs">
			The current app is selected. If the path changes, the path needs to be updated manually.
		</div>
	{/if}
</div>
