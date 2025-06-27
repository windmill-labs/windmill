<script lang="ts">
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Label from '../Label.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import Button from '../common/button/Button.svelte'
	import OneOfInputSpecsEditor from '../apps/editor/settingsPanel/OneOfInputSpecsEditor.svelte'
	import type { AppViewerContext, GridItem, RichConfiguration } from '../apps/types'
	import { findGridItem } from '../apps/editor/appUtils'

	const { selectedComponent, app } = getContext<AppViewerContext>('AppViewerContext')

	type Dataset = {
		value: RichConfiguration
		name: string
		type: 'bar' | 'scatter' | 'line' | 'area' | 'range-bar' | 'range-area'
	}

	let component = $state(undefined) as GridItem | undefined

	$effect(() => {
		if (component === undefined && $selectedComponent && $app) {
			untrack(() => {
				component = findGridItem($app, $selectedComponent[0])
			})
		}
	})

	let isEE = $derived(component?.data?.type === 'agchartscomponentee')

	interface Props {
		value?: Dataset | undefined
		trigger?: import('svelte').Snippet
	}

	let { value = $bindable(undefined), trigger }: Props = $props()

	const dispatch = createEventDispatcher()

	function removeDataset() {
		dispatch('remove')
	}

	const trigger_render = $derived(trigger)
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
			<div class="flex flex-col w-96 gap-4 p-4 max-h-[70vh] overflow-y-auto">
				<Label label="Name">
					<input type="text" bind:value={value.name} />
				</Label>

				<OneOfInputSpecsEditor
					key={'Data'}
					bind:oneOf={value.value}
					id={$selectedComponent?.[0] ?? ''}
					shouldCapitalize={true}
					resourceOnly={false}
					inputSpecsConfiguration={value.value?.['configuration']}
					labels={value.value?.['labels']}
					tooltip={value.value?.['tooltip']}
					disabledOptions={isEE ? [] : ['range-bar', 'range-area']}
				/>

				<Button color="red" size="xs" on:click={removeDataset}>Remove dataset</Button>
			</div>
		{/if}
	{/snippet}
</Popover>
