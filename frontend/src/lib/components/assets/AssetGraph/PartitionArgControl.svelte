<script lang="ts">
	import { resource } from 'runed'
	import { AssetService, type AssetKind, type MaterializedPartition } from '$lib/gen'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { CalendarClock, CircleCheck, CircleDashed, TriangleAlert } from 'lucide-svelte'
	import type { PartitionSpec } from './parsePipelineAnnotations'
	import { untrack } from 'svelte'
	import {
		bucketFromInputValue,
		defaultBucket,
		inputValueFromBucket,
		isBeforeStart,
		partitionInputType,
		partitionMetadataError,
		recentBuckets,
		recentWindow,
		startBucketOf,
		usesCalendarPicker
	} from './partitionBuckets'

	interface Props {
		// The partition cadence parsed from the script's `// partitioned` header.
		spec: PartitionSpec
		// Bound to `args.partition`. `$bindable()` without a default per the
		// AGENTS.md ban on `$bindable(default)` for optional props.
		value?: string | undefined
		workspace: string
		// The `// materialize` ducklake target, if any — enables the
		// "already materialized / recently missing" hints against this
		// producer's own output.
		materializeTarget?: { kind: AssetKind; path: string }
		// Partitioned ducklake inputs (`// on ducklake://…`) — enables the
		// "no upstream data for this bucket yet" hint.
		upstreamAssets?: { kind: AssetKind; path: string }[]
	}

	let {
		value = $bindable(),
		spec,
		workspace,
		materializeTarget,
		upstreamAssets = []
	}: Props = $props()

	const inputType = $derived(partitionInputType(spec))
	const calendarPicker = $derived(usesCalendarPicker(spec))
	// Non-undefined when the `// partitioned` metadata is unusable (bad tz/start).
	const metadataError = $derived(partitionMetadataError(spec))

	// Seed the default bucket on (re)mount. The run form is keyed on the schema
	// AND the partition spec (PipelineScriptView), so a real change to either
	// remounts this and reseeds — an identical re-resolve keeps the user's
	// in-progress pick. `defaultBucket` honours the `start=` anchor (never seeds
	// a pre-start bucket the worker would take verbatim). We deliberately do NOT
	// auto-seed when the metadata is invalid (bad tz/start): the worker takes an
	// explicit `partition` arg verbatim, so seeding one would bypass the
	// backend's own tz/start validation — leave it empty and let the backend
	// surface the error. Dynamic / custom-format specs also stay empty (free
	// text). `untrack` so this fires once per mount, not on every `value` edit.
	$effect(() => {
		untrack(() => {
			value = calendarPicker && !metadataError ? defaultBucket(spec, new Date()) : undefined
		})
	})

	// Whether the producer hasn't reached its `start=` anchor yet — explains why
	// the default bucket is the start rather than today.
	let beforeStart = $derived(calendarPicker && !metadataError && isBeforeStart(spec, new Date()))

	// The native <input> string mirrors `value`; hourly carries an extra minute
	// component that the canonical bucket drops.
	let inputValue = $derived(value ? inputValueFromBucket(spec, value) : '')
	function onNativeInput(raw: string) {
		value = raw ? bucketFromInputValue(spec, raw) : ''
	}

	function listPartitions(kind: AssetKind, path: string) {
		// Only ducklake assets carry materialized-partition rows.
		if (kind !== 'ducklake' || !path) return Promise.resolve([] as MaterializedPartition[])
		return AssetService.listAssetPartitions({ workspace, path }).catch(
			() => [] as MaterializedPartition[]
		)
	}

	// This producer's own materialized partitions — powers the selected-bucket
	// status and the recently-missing list.
	let ownPartitions = resource(
		[() => workspace, () => materializeTarget?.kind, () => materializeTarget?.path],
		async ([ws, kind, path]) => {
			if (!ws || !kind || !path) return [] as MaterializedPartition[]
			return listPartitions(kind, path)
		}
	)
	let materializedSet = $derived(
		new Set(
			(ownPartitions.current ?? [])
				.filter((p) => p.status === 'materialized')
				.map((p) => p.partition)
		)
	)

	let selectedMaterialized = $derived(!!value && materializedSet.has(value))

	// Recently-missing buckets: the last N cadence buckets not yet materialized.
	// Pre-`start=` buckets are dropped — they never materialize on their own, so
	// offering them as run chips would let a click launch a pre-start partition.
	let recentlyMissing = $derived.by(() => {
		if (!materializeTarget || materializeTarget.kind !== 'ducklake' || !calendarPicker) return []
		if (metadataError || ownPartitions.loading) return []
		const startBucket = startBucketOf(spec)
		return recentBuckets(spec, new Date(), recentWindow(spec.kind)).filter(
			(b) => !materializedSet.has(b) && (!startBucket || b >= startBucket)
		)
	})

	// Upstream ducklake inputs, one materialized-set PER input (not unioned).
	// A per-input `tracked` flag (has ≥1 non-empty partition) distinguishes a
	// partitioned input from a whole-table one, so a non-partitioned upstream
	// (single whole-table row) never false-alarms as "no data for this bucket".
	let ducklakeUpstreams = $derived(upstreamAssets.filter((a) => a.kind === 'ducklake'))
	let upstream = resource(
		[() => workspace, () => ducklakeUpstreams.map((a) => a.path).join('\n')],
		async ([ws, joined]) => {
			const paths = joined ? joined.split('\n') : []
			if (!ws || paths.length === 0) return [] as { tracked: boolean; materialized: Set<string> }[]
			return await Promise.all(
				paths.map(async (p) => {
					const rows = await listPartitions('ducklake', p)
					return {
						tracked: rows.some((r) => r.partition !== ''),
						materialized: new Set(
							rows.filter((r) => r.status === 'materialized').map((r) => r.partition)
						)
					}
				})
			)
		}
	)
	// Warn when ANY partition-tracked upstream is missing the selected bucket —
	// checked per-input so a fan-in where one input has the bucket doesn't mask
	// another that lacks it. Never fires while loading or for untracked inputs.
	let upstreamMissing = $derived(
		!!value &&
			calendarPicker &&
			!upstream.loading &&
			(upstream.current ?? []).some((u) => u.tracked && !u.materialized.has(value!))
	)

	const MAX_MISSING_CHIPS = 8
</script>

<div class="flex flex-col gap-1.5">
	<span class="text-xs font-semibold text-emphasis inline-flex items-center gap-1.5">
		<CalendarClock size={13} class="text-fuchsia-600 dark:text-fuchsia-400" />
		Partition
		<span class="text-3xs font-medium text-tertiary px-1 py-0.5 rounded bg-surface-secondary">
			{spec.kind}
		</span>
	</span>

	{#if calendarPicker}
		<input
			type={inputType}
			class="app-editor-input !w-full text-xs"
			value={inputValue}
			onchange={(e) => onNativeInput(e.currentTarget.value)}
		/>
		{#if metadataError}
			<span class="text-3xs inline-flex items-start gap-1 text-amber-700 dark:text-amber-400">
				<TriangleAlert size={12} class="mt-0.5 shrink-0" />
				<span>
					The <span class="font-mono">// partitioned</span> header has {metadataError} — fix it in the
					script; no default is filled in.
				</span>
			</span>
		{:else if beforeStart}
			<span class="text-3xs text-tertiary">
				Partitioning starts <span class="font-mono">{spec.start}</span> — defaulted to the first partition.
			</span>
		{/if}
	{:else}
		<TextInput
			bind:value={() => value ?? '', (v) => (value = v)}
			size="sm"
			inputProps={{
				placeholder: spec.kind === 'dynamic' ? 'Partition key value' : 'Partition bucket'
			}}
		/>
		<span class="text-3xs text-tertiary">
			{#if spec.kind === 'dynamic'}
				Dynamic partition — leave blank to let the run resolve it from the payload.
			{:else}
				Custom partition format — enter the bucket exactly as the producer renders it.
			{/if}
		</span>
	{/if}

	<!-- Selected-bucket status against this producer's own materialized set. -->
	{#if calendarPicker && materializeTarget?.kind === 'ducklake' && value}
		{#if selectedMaterialized}
			<span class="text-3xs inline-flex items-center gap-1 text-green-700 dark:text-green-400">
				<CircleCheck size={12} />
				<span class="font-mono">{value}</span> is already materialized — running replaces it.
			</span>
		{:else}
			<span class="text-3xs inline-flex items-center gap-1 text-tertiary">
				<CircleDashed size={12} />
				<span class="font-mono">{value}</span> not materialized yet — running creates it.
			</span>
		{/if}
	{/if}

	<!-- No upstream data for the selected bucket. -->
	{#if upstreamMissing}
		<span class="text-3xs inline-flex items-start gap-1 text-amber-700 dark:text-amber-400">
			<TriangleAlert size={12} class="mt-0.5 shrink-0" />
			<span>
				No upstream data for <span class="font-mono">{value}</span> yet — this run may materialize an
				empty partition.
			</span>
		</span>
	{/if}

	<!-- Recently-missing buckets, a compact "what's not filled in" hint. -->
	{#if recentlyMissing.length > 0}
		<div class="flex flex-col gap-1">
			<span class="text-3xs text-tertiary">
				{recentlyMissing.length} of the last {recentWindow(spec.kind)}
				{spec.kind} partitions are not materialized:
			</span>
			<div class="flex flex-wrap gap-1">
				{#each recentlyMissing.slice(0, MAX_MISSING_CHIPS) as b (b)}
					<button
						type="button"
						class="px-1.5 py-0.5 rounded text-3xs font-mono bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-300 hover:ring-1 hover:ring-amber-400"
						title="Set the partition to {b}"
						onclick={() => (value = b)}
					>
						{b}
					</button>
				{/each}
				{#if recentlyMissing.length > MAX_MISSING_CHIPS}
					<span class="text-3xs text-tertiary self-center">
						+{recentlyMissing.length - MAX_MISSING_CHIPS} more
					</span>
				{/if}
			</div>
		</div>
	{/if}
</div>
