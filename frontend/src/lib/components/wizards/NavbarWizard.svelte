<script lang="ts">
	import { Popup } from '../common'
	import { userStore, workspaceStore } from '$lib/stores'

	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import type { NavbarItem } from '../apps/editor/component'
	import Label from '../Label.svelte'
	import Toggle from '../Toggle.svelte'
	import { AppService, type ListableApp } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { onMount } from 'svelte'
	import Tooltip from '../Tooltip.svelte'
	import Section from '../Section.svelte'
	import IconSelectInput from '../apps/editor/settingsPanel/inputEditor/IconSelectInput.svelte'
	import Select from '../apps/svelte-select/lib/Select.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from '../DarkModeObserver.svelte'

	export let value: NavbarItem

	let apps: ListableApp[] = []
	let loading = true

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
		loading = false
	}

	onMount(() => {
		loadApps()
	})
	let darkMode: boolean = false
</script>

<DarkModeObserver bind:darkMode />

<Popup
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	containerClasses="border rounded-lg shadow-lg bg-surface p-4"
>
	<svelte:fragment slot="button">
		<slot name="trigger" />
	</svelte:fragment>
	{#if value}
		<Section label="Navbar item" class="flex flex-col gap-2 w-80">
			<Label label="Label">
				<input type="text" bind:value={value.label} />
			</Label>
			<Label label="Path">
				<svelte:fragment slot="header">
					<Tooltip light small>Path to the app</Tooltip>
				</svelte:fragment>

				<Select
					class="grow shrink max-w-full"
					on:change={(e) => {
						value.path = e.detail.value
					}}
					items={apps.map((x) => x.path)}
					placeholder="Pick an path"
					inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
					containerStyles={darkMode
						? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
						: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
					portal={false}
				/>
			</Label>
			<Label label="Caption">
				<input type="text" bind:value={value.caption} />
			</Label>
			<Label label="Disabled">
				<Toggle bind:checked={value.disabled} size="xs" />
			</Label>
			<Label label="Hidden">
				<Toggle bind:checked={value.hidden} size="xs" />
			</Label>
			<Label label="Icon" class="w-full">
				<IconSelectInput bind:value={value.icon} />
			</Label>
		</Section>
	{/if}
</Popup>
