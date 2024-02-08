<script context="module" lang="ts">
	type InsertionMode = 'append' | 'connect' | 'insert'

	type SelectCallback = (path: string) => boolean

	type PropPickerConfig = {
		insertionMode: InsertionMode
		propName: string
		onSelect: SelectCallback
	}

	export type PropPickerWrapperContext = {
		propPickerConfig: Writable<PropPickerConfig | undefined>
		focusProp: (propName: string, insertionMode: InsertionMode, onSelect: SelectCallback) => void
		clearFocus: () => void
	}
</script>

<script lang="ts">
	import PropPicker from '$lib/components/propertyPicker/PropPicker.svelte'
	import PropPickerResult from '$lib/components/propertyPicker/PropPickerResult.svelte'
	import { clickOutside, sendUserToast } from '$lib/utils'
	import { createEventDispatcher, setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'
	import type { PickableProperties } from '../previousResults'
	import { twMerge } from 'tailwind-merge'

	export let pickableProperties: PickableProperties | undefined
	export let result: any = undefined
	export let error: boolean = false
	export let displayContext = true
	export let notSelectable = false
	export let noPadding: boolean = false

	const propPickerConfig = writable<PropPickerConfig | undefined>(undefined)
	const dispatch = createEventDispatcher()

	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		propPickerConfig,
		focusProp: (propName, insertionMode, onSelect) => {
			propPickerConfig.set({
				propName,
				insertionMode,
				onSelect
			})
		},
		clearFocus: () => {
			propPickerConfig.set(undefined)
		}
	})
</script>

<div
	class="h-full w-full"
	use:clickOutside
	on:click_outside={() => propPickerConfig.set(undefined)}
>
	<Splitpanes>
		<Pane
			minSize={20}
			size={60}
			class={twMerge('relative !transition-none', noPadding ? '' : 'p-2')}
		>
			<slot />
		</Pane>
		<Pane
			minSize={20}
			size={40}
			class="pt-2 relative !transition-none {$propPickerConfig ? 'border-2 border-blue-500' : ''}"
		>
			{#if result}
				<PropPickerResult
					{result}
					on:select={({ detail }) => {
						if (!notSelectable && !$propPickerConfig) {
							sendUserToast('Set cursor within an input or click on the plug first', true)
						}
						dispatch('select', detail)
						if ($propPickerConfig?.onSelect(detail)) {
							propPickerConfig.set(undefined)
						}
					}}
				/>
			{:else if pickableProperties}
				<PropPicker
					{displayContext}
					{error}
					{pickableProperties}
					{notSelectable}
					on:select={({ detail }) => {
						if (!notSelectable && !$propPickerConfig) {
							sendUserToast('Set cursor within an input or click on the plug first', true)
						}
						dispatch('select', detail)
						if ($propPickerConfig?.onSelect(detail)) {
							propPickerConfig.set(undefined)
						}
					}}
				/>
			{/if}
		</Pane>
	</Splitpanes>
</div>
