<script lang="ts">
	import { Alert, Button, Skeleton } from '$lib/components/common'
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import { workspaceStore } from '$lib/stores'
	import { Info, X } from 'lucide-svelte'
	import { setGrant } from './access'
	import { detectNeeds, type Needs, type RunnableKind } from './detectNeeds'

	let {
		principal,
		kind,
		path,
		reads,
		onGranted,
		onClose
	}: {
		principal: string
		kind: RunnableKind
		path: string
		/** `kind:path` of every item the principal already reads, through any grant. */
		reads: Set<string>
		onGranted: () => void
		onClose: () => void
	} = $props()

	let needs: Needs | undefined = $state()
	let error: string | undefined = $state()
	let selected: Record<string, boolean> = $state({})
	let results: Record<string, string> = $state({})
	let granting = $state(false)

	// The host keys this component on the runnable, so it detects once per runnable.
	detectNeeds($workspaceStore!, kind, path)
		.then((found) => {
			needs = found
			selected = Object.fromEntries(
				found.items.map((i) => [`${i.kind}:${i.path}`, !reads.has(`${i.kind}:${i.path}`)])
			)
		})
		.catch((e) => (error = `${e?.body ?? e}`))

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
</script>

<div class="flex flex-col gap-3 rounded-md border p-3">
	<div class="flex items-start justify-between gap-2">
		<div class="flex flex-col gap-0.5 min-w-0">
			<span class="text-xs font-semibold text-emphasis truncate">What {path} reads</span>
			<span class="text-2xs text-secondary">
				The resources and variables it reads when it runs, so {principal} can run it. The scripts and
				flows it calls need no grant.
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
	{:else if !needs}
		<Skeleton layout={[[2], 0.5, [2]]} />
	{:else}
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
			<p class="text-xs text-secondary">No resource or variable found in {path}.</p>
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
				A path the code builds while it runs can't be found. Grant it with "Grant".
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
