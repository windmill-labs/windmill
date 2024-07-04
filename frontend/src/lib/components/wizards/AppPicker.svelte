<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import Select from '../apps/svelte-select/lib/Select.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { AppService, type ListableApp } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import type { AppViewerContext } from '../apps/types'
	import Button from '../common/button/Button.svelte'

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
	})
</script>

<DarkModeObserver bind:darkMode />
<div class="flex flex-col gap-2 w-full">
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

	{#if appPath}
		<Button
			size="xs2"
			color="light"
			on:click={() => {
				selecteValue = appPath
				value = appPath
			}}
		>
			Pick current app
		</Button>
	{/if}
</div>
