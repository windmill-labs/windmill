<script lang="ts">
	import type { Snippet } from 'svelte'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		/** The deny button. Render it with `nonCaptureEvent`: the popover's own button takes the click. */
		button: Snippet
		confirmLabel: string
		onDeny: (reason: string | undefined) => void
		disabled?: boolean
	}

	let { button, confirmLabel, onDeny, disabled = false }: Props = $props()

	let reason = $state('')

	function deny(close: () => void) {
		close()
		onDeny(reason.trim() || undefined)
	}
</script>

<Popover
	{disabled}
	placement="bottom-start"
	contentClasses="p-3"
	openFocus="#deny-reason-input"
	triggerAttrs={{ 'aria-label': confirmLabel }}
>
	{#snippet trigger()}
		{@render button()}
	{/snippet}
	{#snippet content({ close })}
		<div class="flex flex-col gap-2 w-72 max-w-[calc(100vw-3rem)]">
			<label class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">
					Reason <span class="font-normal text-secondary">(optional)</span>
				</span>
				<TextInput
					bind:value={reason}
					inputProps={{
						id: 'deny-reason-input',
						maxlength: 1000,
						onkeydown: (e) => e.key === 'Enter' && deny(close)
					}}
				/>
			</label>
			<div class="flex justify-end">
				<Button variant="accent" destructive unifiedSize="sm" onClick={() => deny(close)}>
					{confirmLabel}
				</Button>
			</div>
		</div>
	{/snippet}
</Popover>
