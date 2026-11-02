<script lang="ts">
	import type { Snippet } from 'svelte'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '../common'
	import Tooltip from '../Tooltip.svelte'

	interface Props {
		approveLabel: string
		approveTooltip?: string
		rejectTitle: string
		hideReject?: boolean
		/** Stacks the buttons, for the graph's popover. */
		light?: boolean
		/** Holds the approve button back, e.g. until an answer is typed. */
		approveDisabled?: boolean
		/** Throws to report a failure; the buttons stay usable then. */
		onAnswer: (approved: boolean) => Promise<void>
		/** Shown after the buttons, on the same row. */
		children?: Snippet
	}

	let {
		approveLabel,
		approveTooltip,
		rejectTitle,
		hideReject = false,
		light = false,
		approveDisabled = false,
		onAnswer,
		children
	}: Props = $props()

	let loading = $state(false)
	let answered = $state(false)

	async function answer(approved: boolean) {
		loading = true
		try {
			await onAnswer(approved)
			answered = true
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed', true)
		} finally {
			loading = false
		}
	}
</script>

<div class={twMerge('flex gap-2 items-center', light ? 'flex-col' : 'flex-row')}>
	{#if !hideReject}
		<Button
			title={rejectTitle}
			iconOnly
			startIcon={{ icon: X }}
			variant="default"
			disabled={loading || answered}
			destructive
			unifiedSize="md"
			onClick={() => answer(false)}
		/>
	{/if}
	<Button
		variant="accent"
		onClick={() => answer(true)}
		disabled={loading || answered || approveDisabled}
		unifiedSize="md"
	>
		{approveLabel}
		{#if approveTooltip}
			<Tooltip class="text-white">{approveTooltip}</Tooltip>
		{/if}
	</Button>
	{@render children?.()}
</div>
