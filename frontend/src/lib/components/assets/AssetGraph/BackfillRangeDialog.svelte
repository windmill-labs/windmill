<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import DateInput from '$lib/components/DateInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { resource } from 'runed'
	import { AssetService } from '$lib/gen'
	import { enterpriseLicense } from '$lib/stores'
	import type { BackfillSliceState } from './backfillRun'

	// Range backfill of a partitioned ducklake asset (enterprise). The dialog
	// previews the expected partitions in [from, to] against what
	// `materialized_partition` records (the missing/failed set is the
	// worklist), then hands the worklist to the parent, which runs the
	// producer once per slice (see `backfillRun.ts`). Progress state lives in
	// the parent so a run survives closing the dialog.
	interface Props {
		// Controlled by the parent. `$bindable()` without a default per the
		// AGENTS.md ban on `$bindable(default)` for optional props.
		open?: boolean
		assetPath: string
		workspace: string
		// Live per-slice states of the in-flight (or last finished) backfill.
		slices?: BackfillSliceState[]
		running?: boolean
		// Cancellation was requested but the in-flight slice hasn't finished yet.
		cancelRequested?: boolean
		onStart: (producerPath: string, partitions: string[]) => void
		onCancel: () => void
		// Clear the finished run so the range picker shows again.
		onReset: () => void
	}

	let {
		open = $bindable(),
		assetPath,
		workspace,
		slices,
		running,
		cancelRequested,
		onStart,
		onCancel,
		onReset
	}: Props = $props()

	let fromDate = $state<string | undefined>(undefined)
	let toDate = $state<string | undefined>(undefined)
	let onlyMissing = $state(true)

	let preview = resource(
		[() => workspace, () => assetPath, () => fromDate, () => toDate, () => open ?? false],
		async ([ws, path, from, to, isOpen]) => {
			if (!isOpen || !ws || !path || !from || !to) return undefined
			return await AssetService.listAssetPartitionsInRange({ workspace: ws, path, from, to })
		}
	)

	let worklist = $derived.by(() => {
		const parts = preview.current?.partitions ?? []
		const picked = onlyMissing
			? parts.filter((p) => p.status === 'missing' || p.status === 'failed')
			: parts
		return picked.map((p) => p.partition)
	})
	let counts = $derived.by(() => {
		const c = { missing: 0, materialized: 0, failed: 0, running: 0 }
		for (const p of preview.current?.partitions ?? []) c[p.status]++
		return c
	})
	let canStart = $derived(
		!!$enterpriseLicense && !running && !preview.loading && worklist.length > 0
	)

	// Backend errors come back as a plain-text body (windmill_common::error).
	function errText(e: unknown): string {
		const err = e as { body?: unknown; message?: string }
		if (typeof err?.body === 'string' && err.body) return err.body
		return err?.message ?? String(e)
	}

	const previewChipClass: Record<string, string> = {
		missing: 'bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-300',
		failed: 'bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300',
		materialized: 'bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300',
		running: 'bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300'
	}
	const sliceChipClass: Record<BackfillSliceState['status'], string> = {
		pending: 'bg-surface-secondary text-tertiary',
		running: 'bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300',
		success: 'bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300',
		failure: 'bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300'
	}
</script>

<Modal
	bind:open={() => open ?? false, (v) => (open = v)}
	title={`Backfill ${assetPath}`}
	cancelText="Close"
>
	<div class="flex flex-col gap-4">
		{#if !$enterpriseLicense}
			<p class="text-sm text-secondary">
				Partition backfill is an enterprise feature. Materializing a single partition is available
				in the open-source edition; reprocessing a historical range requires an enterprise license.
			</p>
		{:else if slices?.length}
			<!-- A run is in flight (or just finished): show per-slice progress. -->
			<p class="text-sm text-secondary">
				{#if running && cancelRequested}
					Cancelling — waiting for the current partition to finish; the rest will not run.
				{:else if running}
					Materializing {slices.filter((s) => s.status === 'success' || s.status === 'failure')
						.length}/{slices.length} partitions sequentially — each run gets its partition as an explicit
					arg.
				{:else}
					Backfill finished: {slices.filter((s) => s.status === 'success').length} succeeded,
					{slices.filter((s) => s.status === 'failure').length} failed,
					{slices.filter((s) => s.status === 'pending').length} not run.
				{/if}
			</p>
			<div class="flex flex-col gap-1 max-h-64 overflow-auto">
				{#each slices as s (s.partition)}
					<div class="flex items-center gap-2 text-xs">
						<span class="px-1.5 py-0.5 rounded text-3xs font-medium {sliceChipClass[s.status]}">
							{s.status}
						</span>
						<span class="font-mono">{s.partition}</span>
						{#if s.status === 'running'}
							<Loader2 size={12} class="animate-spin text-tertiary" />
						{/if}
						{#if s.error}
							<span class="text-3xs text-red-600 truncate" title={s.error}>{s.error}</span>
						{/if}
					</div>
				{/each}
			</div>
		{:else}
			<p class="text-sm text-secondary">
				Re-runs the producing script once per partition in the range, each with an explicit
				<span class="font-mono">partition</span> arg. Re-running a partition is idempotent, so this is
				safe to repeat.
			</p>
			<div class="flex gap-3 items-end">
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold">From</span>
					<DateInput bind:value={fromDate} dateFormat="yyyy-MM-dd" />
				</div>
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold">To</span>
					<DateInput bind:value={toDate} dateFormat="yyyy-MM-dd" />
				</div>
			</div>
			{#if preview.loading}
				<div class="flex items-center gap-2 text-tertiary text-xs">
					<Loader2 size={14} class="animate-spin" /> Computing partitions in range…
				</div>
			{:else if preview.error}
				<p class="text-sm text-red-600">{errText(preview.error)}</p>
			{:else if preview.current}
				<div class="flex flex-col gap-2">
					<p class="text-xs text-secondary">
						{preview.current.partitions.length}
						{preview.current.partition_kind} partitions in range — {counts.missing} missing,
						{counts.failed} failed, {counts.materialized} materialized. Producer:
						<span class="font-mono">{preview.current.producer_path}</span>
					</p>
					<div class="flex flex-wrap gap-1 max-h-40 overflow-auto">
						{#each preview.current.partitions as p (p.partition)}
							<span
								class="px-1.5 py-0.5 rounded text-3xs font-mono {previewChipClass[p.status]}"
								title={p.status}
							>
								{p.partition}
							</span>
						{/each}
					</div>
					<Toggle
						bind:checked={onlyMissing}
						size="xs"
						options={{ right: 'Only run missing and failed partitions' }}
					/>
				</div>
			{/if}
		{/if}
	</div>

	{#snippet actions()}
		{#if running}
			<Button variant="default" disabled={cancelRequested} onclick={onCancel}>
				{cancelRequested ? 'Cancelling…' : 'Cancel backfill'}
			</Button>
		{:else if slices?.length}
			<!-- The finished run just changed partition statuses — refresh the
			     range preview along with clearing the run. -->
			<Button
				variant="default"
				onclick={() => {
					onReset()
					preview.refetch()
				}}
			>
				New backfill
			</Button>
		{:else}
			<Button
				variant="accent"
				disabled={!canStart}
				onclick={() => {
					const producer = preview.current?.producer_path
					if (producer) onStart(producer, worklist)
				}}
			>
				Backfill {worklist.length} partition{worklist.length === 1 ? '' : 's'}
			</Button>
		{/if}
	{/snippet}
</Modal>
