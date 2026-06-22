<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import DateInput from '$lib/components/DateInput.svelte'
	import { enterpriseLicense } from '$lib/stores'

	// Backfill re-runs the CE materialization once per partition in [from, to].
	// It is an enterprise feature (orchestration over a range); the dialog is
	// only reachable when licensed, but we guard here too so the action can
	// never fire in CE.
	interface Props {
		// Controlled by the parent. `$bindable()` without a default per the
		// AGENTS.md ban on `$bindable(default)` for optional props.
		open?: boolean
		assetPath: string
		// Invoked with the inclusive ISO date range; the parent performs the
		// actual fan-out (EE backfill endpoint).
		onBackfill: (from: string, to: string) => Promise<void>
	}

	let { open = $bindable(), assetPath, onBackfill }: Props = $props()

	let fromDate = $state<string | undefined>(undefined)
	let toDate = $state<string | undefined>(undefined)
	let loading = $state(false)
	let error = $state<string | undefined>(undefined)

	let canSubmit = $derived(!!$enterpriseLicense && !!fromDate && !!toDate && !loading)

	async function submit() {
		if (!fromDate || !toDate) return
		loading = true
		error = undefined
		try {
			await onBackfill(fromDate, toDate)
			open = false
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
		} finally {
			loading = false
		}
	}
</script>

<Modal bind:open={() => open ?? false, (v) => (open = v)} title={`Backfill ${assetPath}`}>
	<div class="flex flex-col gap-4">
		{#if !$enterpriseLicense}
			<p class="text-sm text-secondary">
				Partition backfill is an enterprise feature. Materializing a single partition is available
				in the open-source edition; reprocessing a historical range requires an enterprise license.
			</p>
		{:else}
			<p class="text-sm text-secondary">
				Re-runs the materialization for each partition in the range. Re-running a partition is
				idempotent, so this is safe to repeat.
			</p>
			<div class="flex gap-3">
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold">From</span>
					<DateInput bind:value={fromDate} />
				</div>
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold">To</span>
					<DateInput bind:value={toDate} />
				</div>
			</div>
			{#if error}
				<p class="text-sm text-red-600">{error}</p>
			{/if}
		{/if}
	</div>

	{#snippet actions()}
		<Button variant="subtle" onclick={() => (open = false)}>Cancel</Button>
		<Button variant="accent" disabled={!canSubmit} {loading} onclick={submit}>
			Start backfill
		</Button>
	{/snippet}
</Modal>
