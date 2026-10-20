<script lang="ts">
	import { Button } from '$lib/components/common'
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
		class?: string
	}

	let {
		toolCallId,
		rejectLabel,
		rejectIcon,
		rejectDestructive = false,
		confirmLabel,
		confirmIcon,
		class: className
	}: Props = $props()

	const chatHost = getChatViewHost()

	function respond(confirmed: boolean) {
		if (toolCallId) {
			chatHost.handleToolConfirmation(toolCallId, confirmed)
		}
	}
</script>

<!-- font-main: the tool cards around it are font-mono, which reads as bold on a button label. -->
<div class={twMerge('flex flex-row items-center justify-end gap-2 font-main', className)}>
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
		startIcon={confirmIcon ? { icon: confirmIcon } : undefined}
		on:click={() => respond(true)}
	>
		{confirmLabel}
	</Button>
</div>
