<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { GranularAclService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { debounce } from '$lib/utils'
	import { Info, Search } from 'lucide-svelte'
	import { setGrant } from './access'
	import { detectNeeds, type Needs, type RunnableKind } from './detectNeeds'

	let {
		principal,
		reads,
		onGranted
	}: {
		principal: string
		/** `kind:path` of every item the principal already reads, through any grant. */
		reads: Set<string>
		onGranted: () => void
	} = $props()

	let runnableKind: RunnableKind = $state('flow')
	let runnablePath: string | undefined = $state()
	let filterText = $state('')
	let candidates: { value: string; label: string }[] = $state([])
	let needs: Needs | undefined = $state()
	let detecting = $state(false)
	let selected: Record<string, boolean> = $state({})
	let results: Record<string, string> = $state({})
	let granting = $state(false)

	async function loadCandidates() {
		const page = await GranularAclService.browseAccessItems({
			workspace: $workspaceStore!,
			principal,
			kind: runnableKind,
			search: filterText.trim() || undefined
		})
		candidates = page.items.map((i) => ({ value: i.path, label: i.path }))
	}
	const loadCandidatesSoon = debounce(loadCandidates, 250).debounced

	function setKind(next: RunnableKind) {
		runnableKind = next
		runnablePath = undefined
		needs = undefined
		void loadCandidates()
	}

	async function detect() {
		if (!runnablePath) return
		detecting = true
		results = {}
		try {
			needs = await detectNeeds($workspaceStore!, runnableKind, runnablePath)
			selected = Object.fromEntries(
				needs.items.map((i) => [`${i.kind}:${i.path}`, !reads.has(`${i.kind}:${i.path}`)])
			)
		} finally {
			detecting = false
		}
	}

	async function grant() {
		if (!needs) return
		granting = true
		for (const item of needs.items) {
			const key = `${item.kind}:${item.path}`
			if (!selected[key]) continue
			try {
				await setGrant($workspaceStore!, item.kind, item.path, principal, 'read')
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

	loadCandidates()
</script>

<div class="flex flex-col gap-3 rounded-md border p-3">
	<div class="flex flex-col gap-0.5">
		<span class="text-xs font-semibold text-emphasis">Add what a flow, script or app reads</span>
		<span class="text-2xs text-secondary">
			Finds the resources and variables it reads when it runs, so {principal} can run it. The scripts
			and flows it calls need no grant.
		</span>
	</div>
	<div class="flex gap-2 items-center">
		<ToggleButtonGroup selected={runnableKind} noWFull onSelected={(v) => setKind(v)}>
			{#snippet children({ item })}
				<ToggleButton value="flow" label="Flow" small {item} />
				<ToggleButton value="script" label="Script" small {item} />
				<ToggleButton value="app" label="App" small {item} />
			{/snippet}
		</ToggleButtonGroup>
		<div class="grow">
			<Select
				items={candidates}
				bind:value={() => runnablePath, (v) => ((runnablePath = v), (needs = undefined))}
				bind:filterText={() => filterText, (v) => ((filterText = v), loadCandidatesSoon())}
				placeholder={runnableKind === 'app' ? 'Pick an app' : `Pick a ${runnableKind}`}
				size="sm"
			/>
		</div>
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: Search }}
			disabled={!runnablePath}
			loading={detecting}
			onClick={detect}
		>
			Find what it reads
		</Button>
	</div>

	{#if needs}
		{#if needs.items.length}
			<div class="flex flex-col divide-y border rounded-md">
				{#each needs.items as item (item.kind + ':' + item.path)}
					{@const key = `${item.kind}:${item.path}`}
					{@const already = reads.has(key)}
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
								{item.kind === 'resource' ? 'Resource' : 'Variable'}, read by {item.sources.join(
									', '
								)}
							</span>
						</span>
						<span class="text-2xs text-secondary shrink-0">
							{results[key] ?? (already ? 'Already reads it' : 'Read')}
						</span>
					</label>
				{/each}
			</div>
		{:else}
			<p class="text-xs text-secondary">
				No resource or variable found in {runnablePath}.
			</p>
		{/if}
		{#if needs.others.length}
			<Alert type="info" size="xs" title="Also used, not granted here">
				{needs.others.map((o) => `${o.kind} ${o.path}`).join(', ')}. Access to these is set on the
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
				A path the code builds while it runs can't be found. Grant it from the list below.
			</span>
			<Button
				variant="accent"
				unifiedSize="sm"
				disabled={selectedCount === 0}
				loading={granting}
				onClick={grant}
			>
				Grant read on {selectedCount}
			</Button>
		</div>
	{/if}
</div>
