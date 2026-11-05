<script lang="ts">
	import type { Snippet } from 'svelte'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '../common'
	import Tooltip from '../Tooltip.svelte'
	import DenyReason from './DenyReason.svelte'

	interface Props {
		approveLabel: string
		approveTooltip?: string
		rejectTitle: string
		hideReject?: boolean
		/** Stacks the buttons, for the graph's popover. */
		light?: boolean
		/** Holds the approve button back, e.g. until an answer is typed. */
		approveDisabled?: boolean
		/** Offers an optional reason field, passed to `onAnswer` on a rejection. */
		rejectReasonPlaceholder?: string
		/** Throws to report a failure; the buttons stay usable then. */
		onAnswer: (approved: boolean, reason?: string) => Promise<void>
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
		rejectReasonPlaceholder,
		onAnswer,
		children
	}: Props = $props()

	let loading = $state(false)
	let answered = $state(false)
	let reason = $state('')

	async function answer(approved: boolean) {
		loading = true
		try {
			await onAnswer(approved, approved ? undefined : reason.trim() || undefined)
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
		{#if rejectReasonPlaceholder}
			<DenyReason
				bind:value={reason}
				class="min-w-48"
				placeholder={rejectReasonPlaceholder}
				disabled={loading || answered}
			/>
		{/if}
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
