<script lang="ts">
	import { Alert, Button, Skeleton } from '$lib/components/common'
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { Info, X } from 'lucide-svelte'
	import { setGrant } from './access'
	import {
		detectNeeds,
		neededByGrantee,
		type NeededItem,
		type Needs,
		type RunnableKind
	} from './detectNeeds'

	const operatingWorkspace = useOperatingWorkspace()

	let {
		principal,
		kind,
		path,
		held,
		onGranted,
		onClose
	}: {
		principal: string
		kind: RunnableKind
		path: string
		/** `kind:path` of every item the principal reaches through any grant, to whether it can write it. */
		held: Map<string, boolean>
		onGranted: () => void
		onClose: () => void
	} = $props()

	const KIND_LABELS: Record<NeededItem['kind'], string> = {
		resource: 'Resource',
		variable: 'Variable',
		script: 'Script',
		flow: 'Flow'
	}

	let needs: Needs | undefined = $state()
	let error: string | undefined = $state()
	let selected: Record<string, boolean> = $state({})
	let results: Record<string, string> = $state({})
	let granting = $state(false)

	const needed = $derived(needs && neededByGrantee(needs, principal))
	/** The account the item's own reads are made as, when it is not the principal. */
	const runsAs = $derived(
		needs?.runs_as.type === 'account' && needs.runs_as.principal !== principal
			? needs.runs_as.principal
			: undefined
	)

	function covered(item: NeededItem): boolean {
		const write = held.get(`${item.kind}:${item.path}`)
		return write !== undefined && (write || item.level === 'read')
	}

	function status(item: NeededItem): string {
		if (covered(item)) return 'Already has it'
		if (held.get(`${item.kind}:${item.path}`) === false) return 'Has read, needs write'
		return item.level === 'write' ? 'Write' : 'Read'
	}

	// The host keys this component on the runnable, so it detects once per runnable.
	detectNeeds($operatingWorkspace!, kind, path)
		.then((found) => {
			needs = found
			selected = Object.fromEntries(
				neededByGrantee(found, principal).items.map((i) => [`${i.kind}:${i.path}`, !covered(i)])
			)
		})
		.catch((e) => (error = `${e?.body ?? e}`))

	async function grant() {
		if (!needed) return
		granting = true
		for (const item of needed.items) {
			const key = `${item.kind}:${item.path}`
			if (!selected[key]) continue
			try {
				await setGrant($operatingWorkspace!, item.kind, item.path, principal, item.level)
				results[key] = 'Granted'
				selected[key] = false
			} catch (e) {
				results[key] = `${e?.body ?? e}`
			}
		}
		granting = false
		onGranted()
	}

	const selectedCount = $derived(Object.values(selected).filter(Boolean).length)
</script>

<div class="flex flex-col gap-3 rounded-md border p-3">
	<div class="flex items-start justify-between gap-2">
		<div class="flex flex-col gap-0.5 min-w-0">
			<span class="text-xs font-semibold text-emphasis truncate">What {path} reads</span>
			<span class="text-2xs text-secondary">
				What {principal} needs to run it: the resources and variables it reads, and the scripts and flows
				its code starts. A step that calls a script or flow by path needs no grant on it.
			</span>
		</div>
		<Button
			variant="subtle"
			unifiedSize="xs"
			iconOnly
			startIcon={{ icon: X }}
			title="Dismiss"
			onClick={onClose}
		/>
	</div>

	{#if error}
		<Alert type="error" size="xs" title="Could not read {path}">{error}</Alert>
	{:else if !needs || !needed}
		<Skeleton layout={[[2], 0.5, [2]]} />
	{:else}
		{#if runsAs}
			<Alert
				type="info"
				size="xs"
				title={kind === 'app' ? `Its runnables run as ${runsAs}` : `Runs as ${runsAs}`}
			>
				{kind === 'app'
					? 'Only what its frontend reads, as whoever opens it, is listed.'
					: `What it reads needs no grant for ${principal}.`}
			</Alert>
		{/if}
		{#if needed.items.length}
			<div class="flex flex-col divide-y border rounded-md">
				{#each needed.items as item (item.kind + ':' + item.path)}
					{@const key = `${item.kind}:${item.path}`}
					{@const already = covered(item)}
					<label class="flex items-start gap-2 px-3 py-1.5 text-xs">
						<Checkbox
							class="mt-0.5"
							checked={selected[key]}
							disabled={already || granting}
							onChange={() => (selected[key] = !selected[key])}
						/>
						<span class="flex flex-col grow min-w-0">
							<span class="font-mono text-2xs text-emphasis truncate">{item.path}</span>
							<span class="text-2xs text-secondary truncate">
								{KIND_LABELS[item.kind]}, used by {item.sources.join(', ')}
							</span>
						</span>
						<span class="text-2xs text-secondary shrink-0">
							{results[key] ?? status(item)}
						</span>
					</label>
				{/each}
			</div>
		{:else}
			<p class="text-xs text-secondary">Nothing {principal} needs was found in {path}.</p>
		{/if}
		{#if needed.others.length}
			<Alert type="info" size="xs" title="Also used, not granted here">
				{needed.others.map((o) => `${o.kind} ${o.path}`).join(', ')}. Access to these is set on the
				storage or data table itself.
			</Alert>
		{/if}
		{#if needs.errors.length}
			<Alert type="warning" size="xs" title="Some of it could not be read">
				{needs.errors.join('; ')}
			</Alert>
		{/if}
		<div class="flex items-center justify-between gap-2">
			<span class="flex items-center gap-1 text-2xs text-secondary">
				<Info size={12} class="shrink-0" />
				Finds paths written literally in its code and inputs. A path built while it runs, passed through
				a helper, or read in a script it imports isn't found: grant it with "Grant".
			</span>
			<Button
				variant="accent"
				unifiedSize="sm"
				disabled={selectedCount === 0}
				loading={granting}
				onClick={grant}
			>
				Grant access to {selectedCount}
			</Button>
		</div>
	{/if}
</div>
