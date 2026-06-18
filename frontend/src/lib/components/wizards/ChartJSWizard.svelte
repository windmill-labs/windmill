<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import Label from '../Label.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import Button from '../common/button/Button.svelte'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import type { AppViewerContext, RichConfiguration } from '../apps/types'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	type Dataset = {
		value: RichConfiguration
		name: string
	}

	interface Props {
		value?: Dataset | undefined
		trigger?: import('svelte').Snippet
	}

	let { value = $bindable(undefined), trigger: trigger_render }: Props = $props()

	const dispatch = createEventDispatcher()

	function removeDataset() {
		dispatch('remove')
	}
</script>

<Popover
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	closeOnOtherPopoverOpen
>
	{#snippet trigger()}
		{@render trigger_render?.()}
	{/snippet}
	{#snippet content()}
		{#if value}
			<div class="flex flex-col w-96 p-4 gap-4 max-h-[70vh] overflow-y-auto">
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
	{/snippet}
</Popover>
