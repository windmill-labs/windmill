<script context="module" lang="ts">
	type SelectCallback = (path: string) => boolean

	export const CONNECT = 'connect' as const
	export type PropPickerConfig = {
		propName?: string
		onSelect: SelectCallback
		clearFocus: () => void
		insertionMode: 'append' | 'connect' | 'insert'
	}

	export type PropPickerWrapperContext = {
		propPickerConfig: Writable<PropPickerConfig | undefined>
		inputMatches: Writable<{ word: string; value: string }[] | undefined>
		focusProp: (
			propName: string,
			insertionMode: 'append' | 'connect' | 'insert',
			onSelect: SelectCallback
		) => void
		clearFocus: () => void
	}
</script>

<script lang="ts">
	import PropPicker from '$lib/components/propertyPicker/PropPicker.svelte'
	import PropPickerResult from '$lib/components/propertyPicker/PropPickerResult.svelte'
	import { clickOutside } from '$lib/utils'
	import { createEventDispatcher, getContext, setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'
	import type { PickableProperties } from '../previousResults'
	import { twMerge } from 'tailwind-merge'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'

	export let pickableProperties: PickableProperties | undefined
	export let result: any = undefined
	export let extraResults: any = undefined
	export let flow_input: any = undefined
	export let error: boolean = false
	export let displayContext = true
	export let notSelectable = false
	export let noPadding: boolean = false
	export let alwaysOn: boolean = false
	export let paneClass: string = ''
	export let noFlowPlugConnect = false

	const propPickerConfig: Writable<PropPickerConfig | undefined> = writable<
		PropPickerConfig | undefined
	>(undefined)

	const inputMatches = writable<{ word: string; value: string }[] | undefined>(undefined)
	const dispatch = createEventDispatcher()

	const { flowPropPickerConfig } = getContext<PropPickerContext>('PropPickerContext')
	flowPropPickerConfig.set(undefined)
	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		propPickerConfig,
		inputMatches,
		focusProp: (propName, insertionMode, onSelect) => {
			const config = {
				propName,
				insertionMode,
				onSelect,
				clearFocus: () => {
					propPickerConfig.set(undefined)
					flowPropPickerConfig.set(undefined)
				}
			}
			propPickerConfig.set(config)
			if (!noFlowPlugConnect) {
				flowPropPickerConfig.set({
					...config,
					clearFocus: () => {
						propPickerConfig.set(undefined)
						flowPropPickerConfig.set(undefined)
					}
				})
			}
		},
		clearFocus: () => {
			flowPropPickerConfig.set(undefined)
			propPickerConfig.set(undefined)
		}
	})

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-prop-picker], [data-prop-picker] *')
		) as HTMLElement[]
	}
</script>

<div
	class="h-full w-full"
	data-prop-picker-root
	use:clickOutside={{ capture: true, exclude: getPropPickerElements }}
	on:click_outside={() => {
		propPickerConfig.set(undefined)
		flowPropPickerConfig.set(undefined)
	}}
>
	<Splitpanes class={$propPickerConfig ? 'splitpanes-remove-splitter' : ''}>
		<Pane
			minSize={20}
			size={60}
			class={twMerge('relative !transition-none ', noPadding ? '' : 'p-2')}
		>
			<slot />
		</Pane>
		<Pane
			minSize={20}
			size={40}
			class="!transition-none z-1000 {$propPickerConfig ? 'ml-[-1px]' : ''} {paneClass}"
		>
			<AnimatedButton
				animate={$propPickerConfig?.insertionMode == 'connect'}
				baseRadius="4px"
				wrapperClasses="prop-picker-inputs h-full w-full pt-1 !bg-surface "
				marginWidth="3px"
				ringColor={$propPickerConfig?.insertionMode == 'insert' ||
				$propPickerConfig?.insertionMode == 'append'
					? '#3b82f6'
					: 'transparent'}
				animationDuration="4s"
			>
				{#if result != undefined}
					<PropPickerResult
						{result}
						{extraResults}
						{flow_input}
						allowCopy={!notSelectable && !$propPickerConfig}
						on:select={({ detail }) => {
							dispatch('select', detail)
							if ($propPickerConfig?.onSelect(detail)) {
								$propPickerConfig?.clearFocus()
							}
						}}
					/>
				{:else if pickableProperties}
					<PropPicker
						{alwaysOn}
						{displayContext}
						{error}
						previousId={pickableProperties?.previousId}
						{pickableProperties}
						allowCopy={!notSelectable && !$propPickerConfig}
						on:select={({ detail }) => {
							// console.log('selecting', detail)
							dispatch('select', detail)
							if ($propPickerConfig?.onSelect(detail)) {
								$propPickerConfig?.clearFocus()
							}
						}}
					/>
				{/if}
			</AnimatedButton>
		</Pane>
	</Splitpanes>
</div>

<style>
	:global(.splitpanes-remove-splitter > .splitpanes__pane) {
		background-color: inherit !important;
	}
	:global(.splitpanes-remove-splitter > .splitpanes__splitter) {
		background-color: transparent !important;
		width: 0 !important;
		border: none !important;
	}
</style>
