<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { resource } from 'runed'
	import { Alert, Drawer, DrawerContent, Section, Skeleton } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Select from './select/Select.svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Cell from './table/Cell.svelte'
	import { formatMemory, msToReadableTime } from '$lib/utils'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	interface Props {
		workerGroups: string[]
	}

	let { workerGroups }: Props = $props()

	const WINDOWS = {
		'24h': 24 * 3600,
		'7d': 7 * 24 * 3600,
		'30d': 30 * 24 * 3600
	}
	const ORDERS = {
		total_duration: 'Total runtime',
		max_memory: 'Max memory',
		cpu: 'CPU time'
	} as const
	type OrderBy = keyof typeof ORDERS
	// The server gathers every run without a stable path (previews, dependency jobs) under this one.
	const ADHOC_PATH = '<adhoc>'

	let drawer: Drawer | undefined = $state()
	let opened = $state(false)
	let windowKey: keyof typeof WINDOWS = $state('24h')
	let orderBy: OrderBy = $state('total_duration')
	let workerGroup: string | undefined = $state()
	let workspace = $state('')

	export function openDrawer(group?: string) {
		workerGroup = group
		opened = true
		drawer?.openDrawer()
		logFeatureUsage('worker_resource_stats', 'opened')
	}

	const stats = resource(
		() => ({ opened, windowKey, orderBy, workerGroup, workspace: workspace.trim() }),
		async (q, _, { signal }) => {
			if (!q.opened) return undefined
			try {
				return await WorkerService.getRunnableStats({
					windowSecs: WINDOWS[q.windowKey],
					orderBy: q.orderBy,
					workerGroup: q.workerGroup || undefined,
					workspace: q.workspace || undefined
				})
			} finally {
				// A slower answer for filters no longer selected must not replace the current one:
				// `resource` drops the abort error thrown in its place.
				signal.throwIfAborted()
			}
		},
		{ debounce: 200 }
	)

	// Many scripts run for well under a second, which `msToReadableTime` rounds to "0.0s".
	function duration(ms: number) {
		return ms < 1000 ? `${Math.round(ms)}ms` : msToReadableTime(ms, 1)
	}

	function cores(cpuMs: number, durationMs: number) {
		return durationMs > 0 ? (cpuMs / durationMs).toFixed(2) : '-'
	}
</script>

<Drawer bind:this={drawer} size="1100px" on:close={() => (opened = false)}>
	<DrawerContent title="Heaviest scripts" on:close={drawer?.closeDrawer}>
		<Section
			label="Resource usage per script"
			tooltip="Aggregated by the workers once an hour, per script and worker group, so the last hour is incomplete. Memory is the peak of the job's main process. CPU time counts that process and the child processes it waited for. Scripts run inside the worker process (SQL, native TypeScript) and jobs of dedicated workers report neither, and a killed job reports no CPU time."
		>
			{#snippet action()}
				<ToggleButtonGroup bind:selected={windowKey} noWFull>
					{#snippet children({ item })}
						{#each Object.keys(WINDOWS) as key (key)}
							<ToggleButton value={key} label={key} size="sm" {item} />
						{/each}
					{/snippet}
				</ToggleButtonGroup>
			{/snippet}

			<div class="flex flex-wrap items-center gap-4 pb-4">
				<div class="flex items-center gap-2">
					<span class="text-xs text-secondary">Sort by</span>
					<ToggleButtonGroup
						bind:selected={
							() => orderBy,
							(v) => {
								orderBy = v
								logFeatureUsage('worker_resource_stats', 'sort', { key: v })
							}
						}
						noWFull
					>
						{#snippet children({ item })}
							{#each Object.entries(ORDERS) as [value, label] (value)}
								<ToggleButton {value} {label} size="sm" {item} />
							{/each}
						{/snippet}
					</ToggleButtonGroup>
				</div>
				<div class="w-48">
					<Select
						items={workerGroups.map((g) => ({ label: g, value: g }))}
						bind:value={workerGroup}
						placeholder="All worker groups"
						clearable
						size="sm"
					/>
				</div>
				<div class="w-48">
					<TextInput
						bind:value={workspace}
						inputProps={{ placeholder: 'Workspace id' }}
						size="sm"
					/>
				</div>
			</div>

			{#if stats.error}
				<Alert type="error" title="Failed to load the script statistics">
					{stats.error.message}
				</Alert>
			{:else if stats.current === undefined}
				<Skeleton layout={[[8]]} />
			{:else if stats.current.length === 0}
				<p class="text-secondary text-xs">No jobs were recorded for this selection.</p>
			{:else}
				<DataTable size="sm" noBorder={false} rounded={true}>
					<Head>
						<tr>
							<Cell head first>Script</Cell>
							<Cell head>Workspace</Cell>
							<Cell head>Worker group</Cell>
							<Cell head numeric>Jobs</Cell>
							<Cell head numeric>Total runtime</Cell>
							<Cell head numeric>Avg runtime</Cell>
							<Cell head numeric>Max memory</Cell>
							<Cell head numeric>Avg memory</Cell>
							<Cell head numeric>CPU time</Cell>
							<Cell head numeric last>Avg CPUs</Cell>
						</tr>
					</Head>
					<tbody>
						{#each stats.current as s (s.workspace_id + '\0' + s.runnable_path + '\0' + s.worker_group)}
							<tr class="border-b last:border-b-0">
								<Cell first class="text-xs text-primary">
									{#if s.runnable_path === ADHOC_PATH}
										<span class="text-secondary">Previews and other ad-hoc runs</span>
									{:else}
										<span class="font-mono">{s.runnable_path}</span>
									{/if}
								</Cell>
								<Cell class="text-xs text-primary">{s.workspace_id}</Cell>
								<Cell class="text-xs text-primary">{s.worker_group}</Cell>
								<Cell numeric class="text-xs text-primary">{s.job_count}</Cell>
								<Cell numeric class="text-xs text-primary">
									{duration(s.total_duration_ms)}
								</Cell>
								<Cell numeric class="text-xs text-primary">
									<!-- A workflow suspended and not yet completed has time recorded and no job. -->
									{s.job_count > 0 ? duration(s.total_duration_ms / s.job_count) : '-'}
								</Cell>
								{#if s.memory_sample_count > 0}
									<Cell numeric class="text-xs text-primary">
										{formatMemory(s.max_memory_peak)}
									</Cell>
									<Cell numeric class="text-xs text-primary">
										{formatMemory(Math.round(s.sum_memory_peak / s.memory_sample_count))}
									</Cell>
								{:else}
									<Cell numeric class="text-xs text-secondary">-</Cell>
									<Cell numeric class="text-xs text-secondary">-</Cell>
								{/if}
								<Cell numeric class="text-xs text-primary">
									{duration(s.total_cpu_ms)}
								</Cell>
								<Cell numeric last class="text-xs text-primary">
									{cores(s.total_cpu_ms, s.total_duration_ms)}
								</Cell>
							</tr>
						{/each}
					</tbody>
				</DataTable>
			{/if}
		</Section>
	</DrawerContent>
</Drawer>
