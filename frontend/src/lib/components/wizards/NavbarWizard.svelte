<script lang="ts">
	import { Popup } from '../common'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import type { NavbarItem } from '../apps/editor/component'
	import Label from '../Label.svelte'
	import { getContext } from 'svelte'
	import Section from '../Section.svelte'
	import IconSelectInput from '../apps/editor/settingsPanel/inputEditor/IconSelectInput.svelte'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import type { AppViewerContext } from '../apps/types'
	import Alert from '../common/alert/Alert.svelte'
	import OneOfInputSpecsEditor from '../apps/editor/settingsPanel/OneOfInputSpecsEditor.svelte'

	export let value: NavbarItem

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
</script>

<Popup
	floatingConfig={{
		strategy: 'absolute',
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

			<OneOfInputSpecsEditor
				key={'Link'}
				bind:oneOf={value.path}
				id={$selectedComponent?.[0] ?? ''}
				shouldCapitalize={true}
				resourceOnly={false}
				inputSpecsConfiguration={value.path?.['configuration']}
				labels={value.path?.['labels']}
				tooltip={value.path?.['tooltip']}
			/>

			<Alert size="xs" title="Link Behavior" collapsible>
				External links starting with http(s) will open in a new tab. Links that include the current
				app will be highlighted. Links pointing to another app will open in the same tab when
				deployed, but in a new tab in the editor.
			</Alert>

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
