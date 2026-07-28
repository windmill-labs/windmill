<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import PropPicker from '$lib/components/propertyPicker/PropPicker.svelte'
	import { Plug } from 'lucide-svelte'
	import { getContext, type Snippet } from 'svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import type { PickableProperties } from '../previousResults'

	// Whether the prop picker is a split pane or a collapsed popover is not this
	// component's choice: it follows PropPickerContext.collapsePropPickerUntilConnect,
	// so a container too narrow for the pane can opt out without a prop.
	interface Props {
		code: string
		label: string
		documentationLink?: string | undefined
		tooltip?: Snippet
		pickableProperties: PickableProperties
		result?: any
		extraResults?: any
		flow_input?: any
		extraLib?: string
		id?: string | undefined
		/** Two-way editor focus state (used by the loop iterator's AI autocomplete). */
		focused?: boolean
		/** The underlying SimpleEditor instance (insertAtCursor, setCode, …). */
		editor?: SimpleEditor | undefined
		/** AI ghost-text preview shown inside the editor. */
		suggestion?: string
		/** Extra buttons rendered in the header (e.g. the AI autocomplete button). */
		headerExtra?: Snippet
		/** Forwarded to the editor container (e.g. the loop iterator's AI onKeyUp). */
		onKeyUp?: (e: KeyboardEvent) => void
		/** Always use the collapsed plug-popover picker (auto-height editor), regardless of
		 *  the sessions-modal context. For expression inputs that live in a tight column. */
		forceCollapsePicker?: boolean
		/** Render read-only and hide the prop-picker connect affordance. Used when the
		 *  owning setting is toggled off but its params are still shown for reference. */
		disabled?: boolean
	}

	let {
		code = $bindable(),
		label,
		documentationLink = undefined,
		tooltip,
		pickableProperties,
		result = undefined,
		extraResults = undefined,
		flow_input = undefined,
		extraLib = undefined,
		id = undefined,
		focused = $bindable(),
		editor = $bindable(),
		suggestion = undefined,
		headerExtra,
		onKeyUp,
		forceCollapsePicker = false,
		disabled = false
	}: Props = $props()

	const propPickerContext = getContext<PropPickerContext | undefined>('PropPickerContext')
	const modalMode = $derived(
		forceCollapsePicker || (propPickerContext?.collapsePropPickerUntilConnect?.() ?? false)
	)

	let pickerPopover: Popover | undefined = $state()

	// SimpleEditor seeds its Monaco model from `code` once, so a `code` that changes
	// underneath it (toggling a setting on writes its seeded expression) would leave the
	// editor showing something the flow no longer holds.
	$effect(() => {
		const next = code ?? ''
		if (editor && next !== editor.getCode()) {
			editor.setCode(next)
		}
	})

	function insert(path: string) {
		if (disabled) return
		editor?.insertAtCursor(path)
		editor?.focus()
	}
</script>

{#snippet editorNode()}
	<div class="relative w-full overflow-clip {modalMode ? '' : 'h-full'}">
		<SimpleEditor
			small
			bind:this={editor}
			bind:code
			on:focus={() => (focused = true)}
			on:blur={() => (focused = false)}
			lang="javascript"
			autoHeight={modalMode}
			class={modalMode ? 'w-full' : 'h-full'}
			shouldBindKey={false}
			{disabled}
			readOnly={disabled}
			{extraLib}
			{suggestion}
		/>
	</div>
{/snippet}

{#snippet header(connect)}
	<div class="mb-2 flex flex-row gap-2 items-center">
		<div class="text-xs font-semibold text-emphasis whitespace-nowrap">
			{label}
			{#if tooltip}<Tooltip {documentationLink}>{@render tooltip()}</Tooltip>{/if}
		</div>
		{@render connect?.()}
		{@render headerExtra?.()}
	</div>
{/snippet}

{#snippet plugPopover()}
	<Popover
		bind:this={pickerPopover}
		placement="bottom-start"
		closeOnOutsideClick
		contentClasses="rounded-md border bg-surface shadow-lg overflow-hidden"
	>
		{#snippet trigger()}
			<Button
				variant="default"
				unifiedSize="sm"
				nonCaptureEvent
				iconOnly
				startIcon={{ icon: Plug }}
				title="Insert a property from a previous step"
			/>
		{/snippet}
		{#snippet content()}
			<div class="max-h-80 w-72 overflow-auto p-2">
				<PropPicker
					{pickableProperties}
					previousId={pickableProperties?.previousId}
					{result}
					{extraResults}
					on:select={({ detail }) => {
						insert(detail)
						pickerPopover?.close()
					}}
				/>
			</div>
		{/snippet}
	</Popover>
{/snippet}

{#if modalMode}
	<!-- Modal: children (header + editor) full-width; picker lives in the plug
	     popover, which stays inside PropPickerWrapper so PropPicker keeps context. -->
	<PropPickerWrapper
		notSelectable
		pickerPopover
		{pickableProperties}
		{result}
		{extraResults}
		{flow_input}
		on:select={({ detail }) => insert(detail)}
		noPadding
	>
		{@render header(disabled ? undefined : plugPopover)}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="border rounded-md overflow-auto w-full" {id} onkeyup={onKeyUp}>
			{@render editorNode()}
		</div>
	</PropPickerWrapper>
{:else}
	{@render header(undefined)}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="border rounded-md overflow-auto w-full" {id} onkeyup={onKeyUp}>
		<PropPickerWrapper
			notSelectable
			forceExpanded
			{pickableProperties}
			{result}
			{extraResults}
			{flow_input}
			on:select={({ detail }) => insert(detail)}
			noPadding
		>
			{@render editorNode()}
		</PropPickerWrapper>
	</div>
{/if}
