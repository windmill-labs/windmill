<script lang="ts">
	import { Button } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	import { getAiChatManager } from './aiChatManagerContext'

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

	const aiChatManager = getAiChatManager()

	function respond(confirmed: boolean) {
		if (toolCallId) {
			aiChatManager.handleToolConfirmation(toolCallId, confirmed)
		}
	}
</script>

<div class={twMerge('flex flex-row items-center justify-end gap-2', className)}>
	<Button
		variant="default"
		size="xs"
		destructive={rejectDestructive}
		startIcon={rejectIcon ? { icon: rejectIcon } : undefined}
		on:click={() => respond(false)}
	>
		{rejectLabel ?? ''}
	</Button>
	<Button
		variant="accent"
		size="xs"
		startIcon={confirmIcon ? { icon: confirmIcon } : undefined}
		on:click={() => respond(true)}
	>
		{confirmLabel}
	</Button>
</div>
