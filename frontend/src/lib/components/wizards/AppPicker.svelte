<script lang="ts">
	import { onMount } from 'svelte'
	import Select from '../apps/svelte-select/lib/Select.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { AppService, type ListableApp } from '$lib/gen'
	import { canWrite } from '$lib/utils'

	export let value = ''
	export let selecteValue = value
	let darkMode = false

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
	})
</script>

<DarkModeObserver bind:darkMode />
<Select
	class="grow shrink max-w-full"
	on:change={(e) => {
		value = e.detail.value
	}}
	bind:value={selecteValue}
	items={apps.map((app) => app.path)}
	placeholder="Pick an app"
	inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
	containerStyles={darkMode
		? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
		: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
	portal={false}
/>
