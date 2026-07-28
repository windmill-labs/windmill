<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ExpressionPicker from '../propPicker/ExpressionPicker.svelte'
	import { type Snippet } from 'svelte'
	import type { PickableProperties } from '../previousResults'

	interface Props {
		code: string
		label: string
		documentationLink?: string | undefined
		tooltip?: Snippet
		pickableProperties: PickableProperties
		result?: any
		extraResults?: any
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
		extraLib = undefined,
		id = undefined,
		focused = $bindable(),
		editor = $bindable(),
		suggestion = undefined,
		headerExtra,
		onKeyUp,
		disabled = false
	}: Props = $props()

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

<div class="flex flex-col">
	<div class="mb-2 flex flex-row gap-2 items-center">
		<div class="text-xs font-semibold text-emphasis whitespace-nowrap">
			{label}
			{#if tooltip}<Tooltip {documentationLink}>{@render tooltip()}</Tooltip>{/if}
		</div>
		{#if !disabled}
			<ExpressionPicker
				id={id ?? label}
				{pickableProperties}
				{result}
				{extraResults}
				onSelect={insert}
			/>
		{/if}
		{@render headerExtra?.()}
	</div>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="border rounded-md overflow-auto w-full" {id} onkeyup={onKeyUp}>
		<div class="relative w-full overflow-clip">
			<SimpleEditor
				small
				bind:this={editor}
				bind:code
				on:focus={() => (focused = true)}
				on:blur={() => (focused = false)}
				lang="javascript"
				autoHeight
				class="w-full"
				shouldBindKey={false}
				{disabled}
				readOnly={disabled}
				{extraLib}
				{suggestion}
			/>
		</div>
	</div>
</div>
