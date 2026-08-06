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
		/** Where the properties are offered, and therefore what a pick does:
		 *  - 'pane': the step's input form — a pick replaces the argument outright.
		 *  - 'sidePane': a settings row — a pick lands at the expression's cursor, since a
		 *    half-written predicate must survive it.
		 *  - 'popover': hangs off each input's own connect button, for hosts with no pane. */
		pickerMode: () => 'pane' | 'sidePane' | 'popover'
		/** The wrapper owns these; nested inputs receive none of their own. */
		pickableProperties: () => PickableProperties | undefined
		/** The step's own result, and anything extra worth offering beside it (a loop's
		 *  `all_iters`). The panes render them directly — in popover mode the picker
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
		connectProp: (propName, onSelect) => connect.arm({ id: propName, onSelect }),
		clearConnect: connect.disarm,
		pickerMode: () => (sidePane ? 'sidePane' : 'pane'),
		pickableProperties: () => pickableProperties,
		result: () => result,
		extraResults: () => extraResults,
		onPick: (path) => dispatch('select', path),
		exprBeingEdited
	})

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-prop-picker], [data-prop-picker] *')
		) as HTMLElement[]
	}

	let rightPaneHeight: number = $state(0)

	// The side column stays put once the row is being worked on: picking a property blurs
	// the editor, so closing on blur would take the column away mid-click.
	let sidePaneOpen = $state(false)
	$effect(() => {
		if ($propPickerConfig != undefined || $exprBeingEdited.length > 0) {
			sidePaneOpen = true
		}
	})
</script>

{#snippet pickerBody()}
	<!-- An armed input owns the pick; with none, it goes to the form's own handler. Both
	     paths must not fire, or a cursor insertion would be applied twice. -->
	{@const deliver = (path: string) =>
		connect.armed ? connect.resolve(path) : dispatch('select', path)}
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
		onClickOutside: () => {
			connect.disarm()
			sidePaneOpen = false
		}
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
					class="shrink-0 w-[38%] min-w-52 max-w-xs h-72 border-l pl-2 {paneClass}"
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
