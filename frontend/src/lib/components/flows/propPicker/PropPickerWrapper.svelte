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
		/** Reveal a `sidePane` column: its input is being written in. Reaching for the editor
		 *  again after dismissing the column has to say so, since an editor that kept focus
		 *  throughout emits no new focus event. */
		openPicker: () => void
		/** Where the properties are offered, and therefore what a pick does:
		 *  - 'pane': the step's input form — a pick replaces the argument outright.
		 *  - 'sidePane': a settings row — a pick lands at the expression's cursor, since a
		 *    half-written predicate must survive it. */
		pickerMode: () => 'pane' | 'sidePane'
		/** The single input a `sidePane` column belongs to. It stays a destination for as
		 *  long as its field is mounted, so a pick lands whether or not a connect is armed —
		 *  a static field has no editor for the host's own `select` handler to write into.
		 *  `undefined` (the setting was switched off) leaves the column with nowhere to
		 *  deliver, so it closes. */
		setPickTarget: (target: { id: string; onSelect: (path: string) => void } | undefined) => void
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
	import { createEventDispatcher, getContext, setContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable, type Writable } from 'svelte/store'
	import type { PickableProperties } from '../previousResults'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { useConnect } from './useConnect.svelte'

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
		 *  form: their picker is a column beside the row, revealed while the expression is
		 *  being written, rather than a permanent split of the panel. */
		sidePane?: boolean
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
		sidePane = false,
		children
	}: Props = $props()

	const propPickerConfig: Writable<PropPickerConfig | undefined> = writable<
		PropPickerConfig | undefined
	>(undefined)

	const inputMatches = writable<{ word: string; value: string }[] | undefined>(undefined)
	const exprBeingEdited = writable<string[]>([])
	const dispatch = createEventDispatcher()

	const propPickerContext = getContext<PropPickerContext>('PropPickerContext')
	const { flowPropPickerConfig } = propPickerContext
	flowPropPickerConfig.set(undefined)

	// The modal panel covers the graph, so a connect armed there could never be resolved by
	// clicking a step — this pane is the only picker in that mode. See `graphParticipates`.
	const inModalPanel = $derived(propPickerContext.inModalPanel?.() ?? false)

	const connect = useConnect({
		inModalPanel: () => inModalPanel,
		hasPickableProperties: () => pickableProperties != undefined,
		flowPropPickerConfig,
		localConfig: propPickerConfig
	})

	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		propPickerConfig,
		inputMatches,
		connectProp: (propName, onSelect) => {
			connect.arm({ id: propName, onSelect })
			openPicker()
		},
		clearConnect: closePicker,
		openPicker,
		pickerMode: () => (sidePane ? 'sidePane' : 'pane'),
		setPickTarget: (target) => {
			pickTarget = target
			if (!target) closePicker()
		},
		onPick: (path) => dispatch('select', path),
		exprBeingEdited
	})

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-prop-picker], [data-prop-picker] *')
		) as HTMLElement[]
	}

	let rightPaneHeight: number = $state(0)

	let pickTarget: { id: string; onSelect: (path: string) => void } | undefined = $state(undefined)

	// Opened and dismissed on demand rather than derived from focus: picking a property blurs
	// the editor, so a column that followed focus would vanish mid-click — and one that
	// latched onto focus would never let go of an editor unmounted by its own setting.
	let sidePaneOpen = $state(false)

	function openPicker() {
		sidePaneOpen = true
	}

	function closePicker() {
		connect.disarm()
		sidePaneOpen = false
	}
</script>

{#snippet pickerBody()}
	<!-- Exactly one destination per pick, or a cursor insertion lands twice: the armed input
	     if there is one, else the column's own input, else the form's `select` handler. -->
	{@const deliver = (path: string) =>
		connect.armed
			? connect.resolve(path)
			: pickTarget
				? pickTarget.onSelect(path)
				: dispatch('select', path)}
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
					on:select={({ detail }) => deliver(detail)}
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
					on:select={({ detail }) => deliver(detail)}
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
		onClickOutside: closePicker
	}}
>
	{#if sidePane}
		<!-- The row keeps its own spacing, since the setting around it lays that out. -->
		<div class="flex w-full items-start gap-3">
			<div class="min-w-0 grow">{@render children?.()}</div>
			{#if sidePaneOpen && (pickableProperties != undefined || result != undefined)}
				<!-- A set height, not the row's: the picker scrolls its own categories, and an
				     expression editor is one line tall next to them. -->
				<div
					class="shrink-0 w-[38%] min-w-52 max-w-xs h-72 overflow-auto border-l pl-2 {paneClass}"
					transition:fade={{ duration: 100 }}
				>
					{@render pickerBody()}
				</div>
			{/if}
		</div>
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
