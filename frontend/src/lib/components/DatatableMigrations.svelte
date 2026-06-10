<script lang="ts">
	import { WorkspaceService, type ListDatatableMigrationsResponse } from '$lib/gen'
	import {
		Loader2,
		ChevronDown,
		ChevronRight,
		Check,
		Clock,
		TriangleAlert,
		Play
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'

	type DatatableMigrationInfo = ListDatatableMigrationsResponse['migrations'][number]

	interface Props {
		workspaceId: string
	}
	let { workspaceId }: Props = $props()

	type DatatableState = {
		name: string
		migrations: DatatableMigrationInfo[]
		appliedStateError: string | null
		loadError?: string
	}

	let loading = $state(true)
	let error: string | undefined = $state(undefined)
	let datatables: DatatableState[] = $state([])
	let expanded: Set<string> = $state(new Set())
	let runningFor: string | undefined = $state(undefined)

	async function load() {
		loading = true
		error = undefined
		datatables = []
		try {
			const tables = await WorkspaceService.listDataTables({ workspace: workspaceId })
			const results: DatatableState[] = []
			for (const t of tables) {
				try {
					const { migrations, applied_state_error } =
						await WorkspaceService.listDatatableMigrations({
							workspace: workspaceId,
							datatable: t.name
						})
					if (migrations.length > 0) {
						results.push({
							name: t.name,
							migrations,
							appliedStateError: applied_state_error ?? null
						})
					}
				} catch (e: any) {
					results.push({
						name: t.name,
						migrations: [],
						appliedStateError: null,
						loadError: e?.message ?? String(e)
					})
				}
			}
			datatables = results
		} catch (e: any) {
			error = e?.body ?? e?.message ?? String(e)
		} finally {
			loading = false
		}
	}

	$effect(() => {
		void workspaceId
		load()
	})

	function pendingCount(dt: DatatableState): number {
		return dt.migrations.filter((m) => !m.applied).length
	}

	function toggle(name: string) {
		const next = new Set(expanded)
		if (next.has(name)) next.delete(name)
		else next.add(name)
		expanded = next
	}

	async function runPending(name: string) {
		runningFor = name
		try {
			const { applied } = await WorkspaceService.runDatatableMigrations({
				workspace: workspaceId,
				datatable: name
			})
			sendUserToast(
				applied.length > 0
					? `Applied ${applied.length} migration${applied.length > 1 ? 's' : ''}`
					: 'No pending migrations to apply'
			)
			await load()
		} catch (e: any) {
			sendUserToast('Failed to run migrations: ' + (e?.message ?? e), true)
		} finally {
			runningFor = undefined
		}
	}
</script>

<h3 class="text-sm font-semibold">Datatable migrations</h3>
{#if loading}
	<div class="flex items-center gap-2 text-xs text-tertiary py-2">
		<Loader2 class="w-4 h-4 animate-spin" /> Loading datatable migrations...
	</div>
{:else if error}
	<div class="text-xs text-red-500 py-2">Failed to load datatable migrations: {error}</div>
{:else if datatables.length > 0}
	<div class="flex flex-col gap-2 mt-3 mb-1">
		{#each datatables as dt (dt.name)}
			{@const pending = pendingCount(dt)}
			<div class="border rounded-md">
				<button
					class="w-full flex items-center justify-between px-3 py-2 hover:bg-surface-hover"
					onclick={() => toggle(dt.name)}
				>
					<span class="text-xs font-medium">{dt.name}</span>
					<div class="flex items-center gap-2 text-2xs text-tertiary">
						{#if pending > 0}
							<span class="text-orange-500">{pending} pending</span>
						{:else}
							<span class="text-green-600">up to date</span>
						{/if}
						{#if expanded.has(dt.name)}
							<ChevronDown class="w-3 h-3" />
						{:else}
							<ChevronRight class="w-3 h-3" />
						{/if}
					</div>
				</button>

				{#if expanded.has(dt.name)}
					<div class="border-t">
						{#if dt.loadError}
							<div class="px-3 py-1.5 text-2xs text-red-500">{dt.loadError}</div>
						{:else}
							{#if dt.appliedStateError}
								<div class="px-3 py-1.5 text-2xs text-orange-500">
									Could not read applied state: {dt.appliedStateError}
								</div>
							{/if}
							<div class="divide-y">
								{#each dt.migrations as m (m.version)}
									<div class="flex items-center gap-2 text-xs px-3 py-1">
										{#if m.drifted}
											<TriangleAlert class="w-3 h-3 text-yellow-500 shrink-0" />
										{:else if m.applied}
											<Check class="w-3 h-3 text-green-600 shrink-0" />
										{:else}
											<Clock class="w-3 h-3 text-orange-500 shrink-0" />
										{/if}
										<span class="font-mono text-2xs text-tertiary">{m.version}</span>
										<span class="grow truncate">{m.name || '(unnamed)'}</span>
										{#if m.drifted}
											<span class="text-2xs text-yellow-600">edited after apply</span>
										{:else if m.applied}
											<span class="text-2xs text-tertiary">applied</span>
										{:else}
											<span class="text-2xs text-orange-500">pending</span>
										{/if}
									</div>
								{/each}
							</div>
							<div class="px-3 py-2 flex justify-end">
								<Button
									size="xs"
									variant="accent"
									startIcon={{ icon: Play }}
									disabled={pending === 0}
									loading={runningFor === dt.name}
									onclick={() => runPending(dt.name)}
								>
									Run pending migrations
								</Button>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	</div>
{:else}
	<span class="text-xs text-secondary"> No datatable migrations </span>
{/if}
