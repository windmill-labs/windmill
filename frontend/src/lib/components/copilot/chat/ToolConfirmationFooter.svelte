<script lang="ts">
	import { Button } from '$lib/components/common'
	import type { Snippet } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { getChatViewHost } from './chatViewHost'

	interface Props {
		toolCallId: string | undefined
		/** Omit the label for an icon-only reject button. */
		rejectLabel?: string
		rejectIcon?: any
		rejectDestructive?: boolean
		confirmLabel: string
		/** Omit for a label-only confirm button. */
		confirmIcon?: any
		confirmDestructive?: boolean
		confirmDisabled?: boolean
		class?: string
		/** Rendered just before the reject button. */
		leading?: Snippet
		/** Runs right before the call is confirmed. */
		onConfirm?: () => void
	}

	let {
		toolCallId,
		rejectLabel,
		rejectIcon,
		rejectDestructive = false,
		confirmLabel,
		confirmIcon,
		confirmDestructive = false,
		confirmDisabled = false,
		class: className,
		leading,
		onConfirm
	}: Props = $props()

	const chatHost = getChatViewHost()

	function respond(confirmed: boolean) {
		if (toolCallId) {
			if (confirmed) onConfirm?.()
			chatHost.handleToolConfirmation(toolCallId, confirmed)
		}
	}
</script>

<!-- font-main: the tool cards around it are font-mono, which reads as bold on a button label. -->
<div class={twMerge('flex flex-row items-center justify-end gap-2 font-main', className)}>
	{@render leading?.()}
	<Button
		variant="default"
		unifiedSize="sm"
		destructive={rejectDestructive}
		startIcon={rejectIcon ? { icon: rejectIcon } : undefined}
		on:click={() => respond(false)}
	>
		{rejectLabel ?? ''}
	</Button>
	<Button
		variant="accent"
		unifiedSize="sm"
		destructive={confirmDestructive}
		disabled={confirmDisabled}
		startIcon={confirmIcon ? { icon: confirmIcon } : undefined}
		on:click={() => respond(true)}
	>
		{confirmLabel}
	</Button>
</div>
