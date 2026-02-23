<script module lang="ts">
	type SelectCallback = (path: string) => boolean

	export const CONNECT = 'connect' as const
	export type PropPickerConfig = {
		propName?: string
		onSelect: SelectCallback
		clearFocus: () => void
	}

	export type PropPickerWrapperContext = {
		propPickerConfig: Writable<PropPickerConfig | undefined>
		inputMatches: Writable<{ word: string; value: string }[] | undefined>
		connectProp: (propName: string, onSelect: SelectCallback) => void
		clearConnect: () => void
		exprBeingEdited: Writable<string[]>
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
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import type { FlowEditorContext } from '../types'

	interface Props {
		pickableProperties: PickableProperties | undefined
		result?: any
		extraResults?: any
		flow_input?: any
		error?: boolean
		displayContext?: boolean
		notSelectable?: boolean
		noPadding?: boolean
		paneClass?: string
		noFlowPlugConnect?: boolean
		children?: import('svelte').Snippet
	}

	let {
		pickableProperties,
		result = undefined,
		extraResults = undefined,
		flow_input = undefined,
		error = false,
		displayContext = true,
		notSelectable = false,
		noPadding = false,
		paneClass = '',
		noFlowPlugConnect = false,
		children
	}: Props = $props()

	const propPickerConfig: Writable<PropPickerConfig | undefined> = writable<
		PropPickerConfig | undefined
	>(undefined)

	const inputMatches = writable<{ word: string; value: string }[] | undefined>(undefined)
	const dispatch = createEventDispatcher()

	const { flowPropPickerConfig } = getContext<PropPickerContext>('PropPickerContext')
	flowPropPickerConfig.set(undefined)

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let flow_env = $derived(pickableProperties?.flow_env || flowStore.val.value.flow_env)
	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		propPickerConfig,
		inputMatches,
		connectProp: (propName, onSelect) => {
			const config = {
				propName,
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
		clearConnect: () => {
			flowPropPickerConfig.set(undefined)
			propPickerConfig.set(undefined)
		},
		exprBeingEdited: writable<string[]>([])
	})

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-prop-picker], [data-prop-picker] *')
		) as HTMLElement[]
	}

	let rightPaneHeight: number = $state(0)
</script>

<div
	class="h-full w-full"
	data-prop-picker-root
	use:clickOutside={{
		capture: true,
		exclude: getPropPickerElements,
		onClickOutside: () => {
			propPickerConfig.set(undefined)
			flowPropPickerConfig.set(undefined)
		}
	}}
>
	<Splitpanes class={$propPickerConfig ? 'splitpanes-remove-splitter' : ''}>
		<Pane minSize={20} size={60} class={'relative !transition-none'}>
			<div style="height: {rightPaneHeight}px;" class={noPadding ? '' : 'p-2'}>
				{@render children?.()}
			</div>
		</Pane>
		<Pane minSize={20} size={40} class="!transition-none z-1000 relative {paneClass}">
			<div bind:clientHeight={rightPaneHeight} class="min-h-40 h-full !bg-surface-secondary">
				<AnimatedButton
					animate={$propPickerConfig != undefined}
					baseRadius="4px"
					wrapperClasses="prop-picker-inputs h-full w-full pt-1"
					marginWidth="3px"
					ringColor="transparent"
					animationDuration="4s"
				>
					{#if result != undefined && !pickableProperties}
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
							{result}
							{extraResults}
							{displayContext}
							{error}
							{flow_env}
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
			</div>
		</Pane>
	</Splitpanes>
</div>
