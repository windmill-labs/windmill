<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import { Popup } from '../common'
	import Label from '../Label.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import Button from '../common/button/Button.svelte'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import type { AppViewerContext, RichConfiguration } from '../apps/types'

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	type Dataset = {
		value: RichConfiguration
		name: string
	}

	export let value: Dataset | undefined = undefined

	const dispatch = createEventDispatcher()

	function removeDataset() {
		dispatch('remove')
	}
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
		<div class="flex flex-col w-96 p-2 gap-4">
			<Label label="Name">
				<input type="text" bind:value={value.name} />
			</Label>

			<InputsSpecEditor
				key={'Data'}
				bind:componentInput={value.value}
				id={$selectedComponent?.[0] ?? ''}
				userInputEnabled={false}
				shouldCapitalize={true}
				resourceOnly={false}
				fieldType={value.value?.['fieldType']}
				subFieldType={value.value?.['subFieldType']}
				format={value.value?.['format']}
				selectOptions={value.value?.['selectOptions']}
				tooltip={value.value?.['tooltip']}
				fileUpload={value.value?.['fileUpload']}
				placeholder={value.value?.['placeholder']}
				customTitle={value.value?.['customTitle']}
				displayType={false}
			/>

			<Button color="red" size="xs" on:click={removeDataset}>Remove dataset</Button>
		</div>
	{/if}
</Popup>
