<script lang="ts">
	import { Popup } from '../common'
	import { userStore, workspaceStore } from '$lib/stores'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import type { NavbarItem } from '../apps/editor/component'
	import Label from '../Label.svelte'
	import Toggle from '../Toggle.svelte'
	import { AppService, type ListableApp } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import Section from '../Section.svelte'
	import IconSelectInput from '../apps/editor/settingsPanel/inputEditor/IconSelectInput.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import type { AppViewerContext } from '../apps/types'
	import Tooltip from '../Tooltip.svelte'

	export let value: NavbarItem

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

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
			<InputsSpecEditor
				key={'Path'}
				bind:componentInput={value.path}
				id={$selectedComponent?.[0] ?? ''}
				userInputEnabled={false}
				shouldCapitalize={true}
				resourceOnly={false}
				fieldType={value.path?.['fieldType']}
				subFieldType={value.path?.['subFieldType']}
				format={value.path?.['format']}
				selectOptions={apps.map((x) => x.path)}
				fileUpload={value.path?.['fileUpload']}
				placeholder={value.path?.['placeholder']}
				customTitle={value.path?.['customTitle']}
				displayType={false}
				tooltip={'Either a static app path or an  expression.'}
			/>

			<Label label="Caption">
				<input type="text" bind:value={value.caption} />
			</Label>
			<Label label="Disabled">
				<Toggle bind:checked={value.disabled} size="xs" />
			</Label>
			<Label label="Hidden">
				<Toggle bind:checked={value.hidden} size="xs" />
			</Label>
			<Label label="On click">
				<svelte:fragment slot="header">
					<Tooltip light small>
						If disabled, the path will be used as a link and will opened in a new tab. If enabled,
						the path will be written in the output.
					</Tooltip>
				</svelte:fragment>
				<Toggle
					bind:checked={value.writeOutputOnClick}
					size="xs"
					options={{
						left: 'Open the path ',
						right: 'Write the path in the output'
					}}
				/>

				{#if !value.writeOutputOnClick}
					<div class="my-1">
						<Alert size="xs" type="info" title="Target">
							In the editor, the app will open in a new tab. In the viewer, it will open in the same
							tab.
						</Alert>
					</div>
				{/if}
			</Label>
			<Label label="Icon" class="w-full">
				<IconSelectInput bind:value={value.icon} />
			</Label>
		</Section>
	{/if}
</Popup>
