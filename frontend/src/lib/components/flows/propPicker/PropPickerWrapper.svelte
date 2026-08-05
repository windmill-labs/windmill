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
		/** 'popover' hangs the picker off each input's own connect button instead of
		 *  taking a pane — for single-argument settings rows, which are not the step's
		 *  input form. */
		pickerMode: () => 'pane' | 'popover'
		/** The wrapper owns these; nested inputs receive none of their own. */
		pickableProperties: () => PickableProperties | undefined
		/** The step's own result, and anything extra worth offering beside it (a loop's
		 *  `all_iters`). Only the pane renders them directly — in popover mode the picker
		 *  hangs off each input, so it reads them from here instead. */
		result: () => any
		extraResults: () => any
		/** Deliver a pick the way the pane does — as a `select` event, so each setting's own
		 *  handler inserts it at the cursor. Replacing the whole value is right for a step
		 *  input but destroys a half-written predicate. */
		onPick: (path: string) => void
		exprBeingEdited: Writable<string[]>
	}
</script>

<script lang="ts">
	import PropPicker from '$lib/components/propertyPicker/PropPicker.svelte'
	import PropPickerResult from '$lib/components/propertyPicker/PropPickerResult.svelte'
	import { clickOutside } from '$lib/utils'
	import { createEventDispatcher, getContext, onDestroy, setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'
	import type { PickableProperties } from '../previousResults'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { useConnect } from './useConnect.svelte'
	import { useFlowEditorTelemetry } from '../flowEditorTelemetry'

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
		/** Settings rows reuse the step-input form for one argument but are not the input
		 *  form; their picker belongs in a popover. */
		popover?: boolean
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
		popover = false,
		children
	}: Props = $props()

	const propPickerConfig: Writable<PropPickerConfig | undefined> = writable<
		PropPickerConfig | undefined
	>(undefined)

	const inputMatches = writable<{ word: string; value: string }[] | undefined>(undefined)
	const dispatch = createEventDispatcher()

	const propPickerContext = getContext<PropPickerContext>('PropPickerContext')
	const { flowPropPickerConfig } = propPickerContext
	flowPropPickerConfig.set(undefined)

	// The modal panel covers the graph, so a connect armed there could never be resolved by
	// clicking a step — this pane is the only picker in that mode. See `graphParticipates`.
	const inModalPanel = $derived(propPickerContext.inModalPanel?.() ?? false)

	const telemetry = useFlowEditorTelemetry()
	const connect = useConnect({
		inModalPanel: () => inModalPanel,
		hasPickableProperties: () => pickableProperties != undefined,
		flowPropPickerConfig,
		localConfig: propPickerConfig,
		onEvent: (event) => telemetry.log('connect', `input:${event}`)
	})

	// An armed target that goes away with the panel is abandoned like any other, and has to
	// say so or `open` never balances against `insert` + `abandon`. `disarm` no-ops unless
	// this surface is the one holding the arm.
	onDestroy(() => connect.disarm())

	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		propPickerConfig,
		inputMatches,
		connectProp: (propName, onSelect) => connect.arm({ id: propName, onSelect }),
		clearConnect: connect.disarm,
		pickerMode: () => (popover ? 'popover' : 'pane'),
		pickableProperties: () => pickableProperties,
		result: () => result,
		extraResults: () => extraResults,
		onPick: (path) => dispatch('select', path),
		exprBeingEdited: writable<string[]>([])
	})

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-prop-picker], [data-prop-picker] *')
		) as HTMLElement[]
	}

	let rightPaneHeight: number = $state(0)
</script>

{#snippet pickerBody()}
	<div bind:clientHeight={rightPaneHeight} class="min-h-40 h-full !bg-surface">
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
						connect.resolve(detail)
					}}
				/>
			{:else if pickableProperties}
				<PropPicker
					{result}
					{extraResults}
					{displayContext}
					{error}
					previousId={pickableProperties?.previousId}
					{pickableProperties}
					allowCopy={!notSelectable && !$propPickerConfig}
					on:select={({ detail }) => {
						dispatch('select', detail)
						connect.resolve(detail)
					}}
				/>
			{/if}
		</AnimatedButton>
	</div>
{/snippet}

<div
	class="h-full w-full"
	data-prop-picker-root
	use:clickOutside={{
		capture: true,
		exclude: getPropPickerElements,
		// Through the controller, not the stores: it owns the armed target, and a
		// target left armed here would make the next click on that same input
		// read as a toggle-off.
		onClickOutside: connect.disarm
	}}
>
	{#if popover}
		<!-- The picker lives on each input's connect button here, so the row keeps its
		     full width — and its own spacing, since the setting around it lays that out. -->
		{@render children?.()}
	{:else}
		<Splitpanes class={$propPickerConfig ? 'splitpanes-remove-splitter' : ''}>
			<Pane minSize={20} size={60} class={'relative !transition-none'}>
				<div style="height: {rightPaneHeight}px;" class={noPadding ? '' : 'p-2'}>
					{@render children?.()}
				</div>
			</Pane>
			<Pane minSize={20} size={40} class="!transition-none z-1000 relative {paneClass}">
				{@render pickerBody()}
			</Pane>
		</Splitpanes>
	{/if}
</div>
