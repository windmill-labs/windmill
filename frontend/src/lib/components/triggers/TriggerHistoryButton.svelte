<script lang="ts">
	import { History } from 'lucide-svelte'
	import { TriggerService, type TriggerHistoryEntry } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import Button from '../common/button/Button.svelte'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Badge from '../common/badge/Badge.svelte'
	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import TriggerHistoryChanges from './TriggerHistoryChanges.svelte'
	import { getTriggerWorkspace } from './triggerWorkspace'
	import type { TriggerType } from './utils'

	interface Props {
		/** Trigger kind as the backend records it: `schedule`, `http`, `kafka`, … */
		triggerKind: TriggerType
		path: string
	}

	let { triggerKind, path }: Props = $props()

	// An AI session can edit a trigger in a workspace that is not the nav one;
	// the whole trigger subtree reads its workspace through this seam.
	const triggerWs = getTriggerWorkspace()
	const wsId = $derived(triggerWs?.() ?? $workspaceStore)

	let drawer: Drawer | undefined = $state()
	let entries: TriggerHistoryEntry[] | undefined = $state(undefined)
	let loading = $state(false)
	let error: string | undefined = $state(undefined)

	const PER_PAGE = 50

	async function load() {
		if (!wsId) return
		loading = true
		error = undefined
		try {
			entries = await TriggerService.listTriggerHistory({
				workspace: wsId,
				triggerKind,
				path,
				perPage: PER_PAGE
			})
		} catch (e) {
			error = e?.body ?? e?.message ?? 'Could not load history'
			entries = []
		} finally {
			loading = false
		}
	}

	// `worker` is the one value that is not a person: the server disabled the
	// trigger on its own, so it reads as a warning rather than as attribution.
	const sourceColor = {
		ui: 'blue',
		cli: 'gray',
		api: 'indigo',
		worker: 'yellow'
	} as const

	const operationColor = {
		create: 'green',
		update: 'blue',
		delete: 'red',
		enable: 'green',
		disable: 'red',
		suspend: 'yellow'
	} as const
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Trigger history" on:close={() => drawer?.closeDrawer()}>
		{#if loading}
			<div class="flex flex-col gap-2">
				{#each new Array(3) as _, i (i)}
					<Skeleton layout={[[4], 0.7]} />
				{/each}
			</div>
		{:else if error}
			<p class="text-sm text-red-500">{error}</p>
		{:else if !entries || entries.length === 0}
			<p class="text-sm text-secondary">No modification recorded yet</p>
		{:else}
			<div class="flex flex-col gap-2">
				{#each entries as entry (entry.id)}
					<div class="flex flex-col gap-2 border border-border-light rounded-md p-3">
						<div class="flex flex-row gap-2 items-center flex-wrap">
							<Badge color={operationColor[entry.operation] ?? 'gray'}>{entry.operation}</Badge>
							<Badge color={sourceColor[entry.source] ?? 'gray'}>{entry.source}</Badge>
							<span class="text-sm text-primary">
								{entry.username ?? 'Windmill'}
							</span>
							<span class="text-xs text-secondary ml-auto">{displayDate(entry.created_at)}</span>
						</div>
						{#if entry.changes}
							<TriggerHistoryChanges changes={entry.changes} />
						{/if}
					</div>
				{/each}
			</div>
			{#if entries.length === PER_PAGE}
				<p class="text-xs text-secondary mt-2">
					Showing the {PER_PAGE} most recent modifications
				</p>
			{/if}
		{/if}
	</DrawerContent>
</Drawer>

<Button
	unifiedSize="sm"
	variant="subtle"
	startIcon={{ icon: History }}
	iconOnly
	title="Modification history"
	on:click={() => {
		drawer?.openDrawer()
		load()
	}}
/>
