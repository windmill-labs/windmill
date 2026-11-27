<script lang="ts">
	import { Minus, Plus } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		/** Current scale, 1 is actual size; undefined while there is no image. */
		scale: number | undefined
		min: number
		max: number
		onZoomOut: () => void
		onZoomIn: () => void
		/** A scale typed by the user, already clamped to `min`..`max`. */
		onZoomTo: (scale: number) => void
	}

	let { scale, min, max, onZoomOut, onZoomIn, onZoomTo }: Props = $props()

	const percent = $derived(scale === undefined ? '' : `${Math.round(scale * 100)}%`)
	let draft = $state<string | undefined>(undefined)

	function edit() {
		if (scale !== undefined) draft = String(Math.round(scale * 100))
	}

	function commit() {
		if (draft === undefined) return
		const typed = parseFloat(draft.replace('%', '').trim())
		draft = undefined
		if (Number.isFinite(typed) && typed > 0) {
			onZoomTo(Math.min(max, Math.max(min, typed / 100)))
		}
	}

	function onKeyDown(e: KeyboardEvent) {
		// The viewers bind +, -, 0 and 1 as zoom shortcuts; typed digits must not reach them.
		e.stopPropagation()
		if (e.key === 'Enter') commit()
		else if (e.key === 'Escape') {
			e.preventDefault()
			draft = undefined
		}
	}

	function focusAndSelect(node: HTMLElement) {
		const input = node.querySelector('input')
		input?.focus()
		input?.select()
	}
</script>

<div class="flex items-center gap-0.5">
	{#if draft !== undefined}
		<div class="w-14" {@attach focusAndSelect}>
			<TextInput
				size="sm"
				bind:value={draft}
				class="text-center tabular-nums"
				inputProps={{
					'aria-label': 'Zoom level in percent',
					inputmode: 'decimal',
					onkeydown: onKeyDown,
					onblur: commit
				}}
			/>
		</div>
	{:else}
		<Button
			unifiedSize="sm"
			variant="subtle"
			btnClasses="w-14 justify-center tabular-nums"
			title="Set zoom level"
			disabled={scale === undefined}
			onClick={edit}
		>
			{percent}
		</Button>
	{/if}
	<Button
		unifiedSize="sm"
		variant="subtle"
		iconOnly
		startIcon={{ icon: Minus }}
		title="Zoom out (-)"
		disabled={scale === undefined || scale <= min + 1e-3}
		onClick={onZoomOut}
	/>
	<Button
		unifiedSize="sm"
		variant="subtle"
		iconOnly
		startIcon={{ icon: Plus }}
		title="Zoom in (+)"
		disabled={scale === undefined || scale >= max - 1e-3}
		onClick={onZoomIn}
	/>
</div>
