<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { AppService, type ListableApp } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import type { AppViewerContext } from '../apps/types'
	import Alert from '../common/alert/Alert.svelte'
	import Select from '../select/Select.svelte'

	export let value = ''
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
		if (value === '') value = $appPath
	})
</script>

<DarkModeObserver bind:darkMode />

<div class="flex flex-col gap-2 w-full">
	<Select
		clearable
		onClear={() => (value = '')}
		bind:value
		items={apps.map((app) => ({
			value: app.path,
			label: app.path === $appPath ? `${app.path} (current app)` : app.path
		}))}
		placeholder="Pick an app"
		disablePortal
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
