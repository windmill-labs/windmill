<script lang="ts">
	import { Popup } from '../common'
	import { userStore, workspaceStore } from '$lib/stores'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import type { NavbarItem } from '../apps/editor/component'
	import Label from '../Label.svelte'
	import { AppService, type ListableApp } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import Section from '../Section.svelte'
	import IconSelectInput from '../apps/editor/settingsPanel/inputEditor/IconSelectInput.svelte'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import type { AppViewerContext } from '../apps/types'
	import AppPicker from './AppPicker.svelte'

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
	shouldUsePortal={false}
>
	<svelte:fragment slot="button">
		<slot name="trigger" />
	</svelte:fragment>
	{#if value}
		<Section label="Navbar item" class="flex flex-col gap-2 w-80">
			<InputsSpecEditor
				key={'Label'}
				bind:componentInput={value.label}
				id={$selectedComponent?.[0] ?? ''}
				userInputEnabled={false}
				shouldCapitalize={true}
				resourceOnly={false}
				fieldType={value.label?.['fieldType']}
				subFieldType={value.label?.['subFieldType']}
				format={value.label?.['format']}
				selectOptions={value.label?.['selectOptions']}
				fileUpload={value.label?.['fileUpload']}
				placeholder={value.label?.['placeholder']}
				customTitle={value.label?.['customTitle']}
				displayType={false}
			/>

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
				selectOptions={value.path?.['selectOptions']}
				fileUpload={value.path?.['fileUpload']}
				placeholder={value.path?.['placeholder']}
				customTitle={value.path?.['customTitle']}
				displayType={false}
				tooltip={'Either a static app path or an  expression.'}
			/>

			<AppPicker
				items={apps.map((app) => app.path)}
				on:pick={(path) => {
					value.path = {
						type: 'static',
						value: path.detail.value,
						fieldType: 'text'
					}
				}}
			/>

			<Label label="Caption">
				<input type="text" bind:value={value.caption} />
			</Label>
			<InputsSpecEditor
				key={'Disabled'}
				bind:componentInput={value.disabled}
				id={$selectedComponent?.[0] ?? ''}
				userInputEnabled={false}
				shouldCapitalize={true}
				resourceOnly={false}
				fieldType={value.disabled?.['fieldType']}
				subFieldType={value.disabled?.['subFieldType']}
				format={value.disabled?.['format']}
				selectOptions={value.disabled?.['selectOptions']}
				fileUpload={value.disabled?.['fileUpload']}
				placeholder={value.disabled?.['placeholder']}
				customTitle={value.disabled?.['customTitle']}
				displayType={false}
			/>
			<InputsSpecEditor
				key={'Hidden'}
				bind:componentInput={value.hidden}
				id={$selectedComponent?.[0] ?? ''}
				userInputEnabled={false}
				shouldCapitalize={true}
				resourceOnly={false}
				fieldType={value.hidden?.['fieldType']}
				subFieldType={value.hidden?.['subFieldType']}
				format={value.hidden?.['format']}
				selectOptions={value.hidden?.['selectOptions']}
				fileUpload={value.hidden?.['fileUpload']}
				placeholder={value.hidden?.['placeholder']}
				customTitle={value.hidden?.['customTitle']}
				displayType={false}
			/>

			<Label label="Icon" class="w-full">
				<IconSelectInput
					bind:value={value.icon}
					floatingConfig={{
						strategy: 'fixed',
						placement: 'left-end',
						middleware: [offset(8), flip(), shift()]
					}}
					shouldUsePortal={false}
				/>
			</Label>
		</Section>
	{/if}
</Popup>
