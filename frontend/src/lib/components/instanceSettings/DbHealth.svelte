<script lang="ts">
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, RefreshCw, ChevronDown, ChevronRight } from 'lucide-svelte'

	let loading = $state(false)
	let data: any = $state(null)
	let error: string | null = $state(null)
	let scanLimit = $state(10000)

	const scanLimitOptions = [
		{ label: '10,000', value: 10000 },
		{ label: '50,000', value: 50000 },
		{ label: '100,000', value: 100000 },
		{ label: '500,000', value: 500000 }
	]

	let expandedSections: Record<string, boolean> = $state({
		database_size: true,
		job_retention: true,
		large_results: true,
		connection_pool: true,
		table_maintenance: true,
		slow_queries: true,
		datatables: true
	})

	function toggleSection(key: string) {
		expandedSections[key] = !expandedSections[key]
	}

	async function runDiagnostics() {
		loading = true
		error = null
		try {
			const response = await fetch(`/api/db_health?scan_limit=${scanLimit}`, {
				credentials: 'include'
			})
			if (!response.ok) {
				const text = await response.text()
				throw new Error(text || `HTTP ${response.status}`)
			}
			data = await response.json()
		} catch (e: any) {
			error = e.message
			sendUserToast('Failed to run diagnostics: ' + e.message, true)
		} finally {
			loading = false
		}
	}

	function statusColor(status: string): string {
		switch (status) {
			case 'green':
				return 'text-green-600 dark:text-green-400'
			case 'yellow':
				return 'text-yellow-600 dark:text-yellow-400'
			case 'red':
				return 'text-red-600 dark:text-red-400'
			default:
				return 'text-tertiary'
		}
	}

	function statusBadge(status: string): string {
		switch (status) {
			case 'green':
				return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
			case 'yellow':
				return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
			case 'red':
				return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
			default:
				return 'bg-surface-secondary text-secondary'
		}
	}

	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 B'
		const k = 1024
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
		const i = Math.floor(Math.log(bytes) / Math.log(k))
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
	}

	function formatNumber(n: number): string {
		return n.toLocaleString()
	}

	function formatMs(ms: number): string {
		if (ms < 1) return '<1 ms'
		if (ms < 1000) return ms.toFixed(1) + ' ms'
		return (ms / 1000).toFixed(2) + ' s'
	}

	function formatDate(d: string | null): string {
		if (!d) return 'Never'
		return new Date(d).toLocaleString()
	}
</script>

<div class="flex flex-col gap-4">
	<div class="flex items-center gap-2">
		<Button
			variant="default"
			onclick={runDiagnostics}
			disabled={loading}
			startIcon={{ icon: loading ? Loader2 : RefreshCw }}
		>
			{loading ? 'Running...' : 'Run Diagnostics'}
		</Button>
		<label class="text-tertiary flex items-center gap-1 text-xs">
			Scan last
			<select
				class="border-surface-secondary text-secondary rounded border bg-transparent px-1 py-0.5 text-xs"
				bind:value={scanLimit}
			>
				{#each scanLimitOptions as opt}
					<option value={opt.value}>{opt.label}</option>
				{/each}
			</select>
			jobs
		</label>
		{#if loading}
			<span class="text-tertiary text-xs">This may take a few seconds...</span>
		{/if}
	</div>

	{#if error}
		<div
			class="rounded border border-red-300 bg-red-50 p-3 text-sm text-red-700 dark:border-red-700 dark:bg-red-950 dark:text-red-300"
		>
			{error}
		</div>
	{/if}

	{#if data}
		<!-- Database Size -->
		<section class="border-surface-secondary rounded-md border">
			<button
				class="flex w-full items-center justify-between p-3 text-left hover:bg-surface-secondary/50"
				onclick={() => toggleSection('database_size')}
			>
				<h3 class="text-primary text-sm font-semibold">Database Size</h3>
				<div class="flex items-center gap-2">
					<span class="text-tertiary text-xs">{data.database_size.total_size_pretty}</span>
					{#if expandedSections.database_size}
						<ChevronDown size={16} />
					{:else}
						<ChevronRight size={16} />
					{/if}
				</div>
			</button>
			{#if expandedSections.database_size}
				<div class="border-surface-secondary border-t p-3">
					<p class="text-secondary mb-2 text-xs">
						Total database size: <strong>{data.database_size.total_size_pretty}</strong>
					</p>
					<div class="overflow-x-auto">
						<table class="w-full text-left text-xs">
							<thead>
								<tr class="text-tertiary border-surface-secondary border-b">
									<th class="pb-1 pr-4">Table</th>
									<th class="pb-1 pr-4 text-right">Size</th>
								</tr>
							</thead>
							<tbody>
								{#each data.database_size.top_tables as t}
									<tr class="border-surface-secondary border-b last:border-0">
										<td class="text-primary py-1 pr-4 font-mono">{t.table_name}</td>
										<td class="text-secondary py-1 pr-4 text-right">{t.total_size_pretty}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</div>
			{/if}
		</section>

		<!-- Job Retention -->
		<section class="border-surface-secondary rounded-md border">
			<button
				class="flex w-full items-center justify-between p-3 text-left hover:bg-surface-secondary/50"
				onclick={() => toggleSection('job_retention')}
			>
				<h3 class="text-primary text-sm font-semibold">Job Retention</h3>
				<div class="flex items-center gap-2">
					<span
						class="rounded px-1.5 py-0.5 text-xs font-medium {statusBadge(
							data.job_retention.status
						)}"
					>
						{data.job_retention.status}
					</span>
					{#if expandedSections.job_retention}
						<ChevronDown size={16} />
					{:else}
						<ChevronRight size={16} />
					{/if}
				</div>
			</button>
			{#if expandedSections.job_retention}
				<div class="border-surface-secondary flex flex-col gap-1 border-t p-3 text-xs">
					<p class="text-secondary">
						Total completed jobs: <strong
							>{formatNumber(data.job_retention.total_completed_jobs)}</strong
						>
					</p>
					<p class="text-secondary">
						Oldest job: <strong>{formatDate(data.job_retention.oldest_completed_at)}</strong>
					</p>
					<p class="text-secondary">
						Retention period: <strong>
							{data.job_retention.retention_period_secs
								? formatNumber(data.job_retention.retention_period_secs) + 's'
								: 'Not configured'}
						</strong>
					</p>
					<p class="{statusColor(data.job_retention.status)} mt-1 font-medium">
						{data.job_retention.message}
					</p>
				</div>
			{/if}
		</section>

		<!-- Large Results -->
		<section class="border-surface-secondary rounded-md border">
			<button
				class="flex w-full items-center justify-between p-3 text-left hover:bg-surface-secondary/50"
				onclick={() => toggleSection('large_results')}
			>
				<h3 class="text-primary text-sm font-semibold"
					>Large Job Results (last {scanLimit.toLocaleString()} jobs)</h3
				>
				<div class="flex items-center gap-2">
					{#if data.large_results.avg_result_size_bytes != null}
						<span class="text-tertiary text-xs"
							>avg: {formatBytes(data.large_results.avg_result_size_bytes)}</span
						>
					{/if}
					{#if expandedSections.large_results}
						<ChevronDown size={16} />
					{:else}
						<ChevronRight size={16} />
					{/if}
				</div>
			</button>
			{#if expandedSections.large_results}
				<div class="border-surface-secondary border-t p-3">
					{#if data.large_results.top_large_results.length === 0}
						<p class="text-tertiary text-xs">No job results found in the last 30 days.</p>
					{:else}
						<div class="overflow-x-auto">
							<table class="w-full text-left text-xs">
								<thead>
									<tr class="text-tertiary border-surface-secondary border-b">
										<th class="pb-1 pr-4">Job ID</th>
										<th class="pb-1 pr-4">Workspace</th>
										<th class="pb-1 pr-4">Script</th>
										<th class="pb-1 pr-4 text-right">Result Size</th>
										<th class="pb-1 pr-4 text-right">Completed</th>
									</tr>
								</thead>
								<tbody>
									{#each data.large_results.top_large_results as r}
										<tr class="border-surface-secondary border-b last:border-0">
											<td class="text-primary py-1 pr-4 font-mono">{r.id.substring(0, 8)}...</td>
											<td class="text-secondary py-1 pr-4">{r.workspace_id}</td>
											<td class="text-secondary py-1 pr-4 font-mono">{r.runnable_path ?? '-'}</td>
											<td class="text-secondary py-1 pr-4 text-right"
												>{formatBytes(r.result_size_bytes)}</td
											>
											<td class="text-secondary py-1 pr-4 text-right"
												>{formatDate(r.completed_at)}</td
											>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				</div>
			{/if}
		</section>

		<!-- Connection Pool -->
		<section class="border-surface-secondary rounded-md border">
			<button
				class="flex w-full items-center justify-between p-3 text-left hover:bg-surface-secondary/50"
				onclick={() => toggleSection('connection_pool')}
			>
				<h3 class="text-primary text-sm font-semibold">Connection Pool</h3>
				<div class="flex items-center gap-2">
					<span
						class="rounded px-1.5 py-0.5 text-xs font-medium {statusBadge(
							data.connection_pool.status
						)}"
					>
						{data.connection_pool.status}
					</span>
					{#if expandedSections.connection_pool}
						<ChevronDown size={16} />
					{:else}
						<ChevronRight size={16} />
					{/if}
				</div>
			</button>
			{#if expandedSections.connection_pool}
				<div class="border-surface-secondary flex flex-col gap-1 border-t p-3 text-xs">
					<p class="text-secondary">
						Pool size: <strong>{data.connection_pool.pool.size}</strong> / Max:
						<strong>{data.connection_pool.pool.max_connections}</strong>
						/ Idle: <strong>{data.connection_pool.pool.idle}</strong>
					</p>
					<p class="text-secondary">
						Active PG connections: <strong>{data.connection_pool.pg_active_connections}</strong>
					</p>
					<p class="{statusColor(data.connection_pool.status)} mt-1 font-medium">
						{data.connection_pool.message}
					</p>
				</div>
			{/if}
		</section>

		<!-- Table Maintenance -->
		<section class="border-surface-secondary rounded-md border">
			<button
				class="flex w-full items-center justify-between p-3 text-left hover:bg-surface-secondary/50"
				onclick={() => toggleSection('table_maintenance')}
			>
				<h3 class="text-primary text-sm font-semibold">Table Maintenance (Vacuum/Bloat)</h3>
				<div class="flex items-center gap-2">
					{#if expandedSections.table_maintenance}
						<ChevronDown size={16} />
					{:else}
						<ChevronRight size={16} />
					{/if}
				</div>
			</button>
			{#if expandedSections.table_maintenance}
				<div class="border-surface-secondary border-t p-3">
					<div class="overflow-x-auto">
						<table class="w-full text-left text-xs">
							<thead>
								<tr class="text-tertiary border-surface-secondary border-b">
									<th class="pb-1 pr-4">Table</th>
									<th class="pb-1 pr-4 text-right">Live Tuples</th>
									<th class="pb-1 pr-4 text-right">Dead Tuples</th>
									<th class="pb-1 pr-4 text-right">Dead %</th>
									<th class="pb-1 pr-4 text-right">Last Vacuum</th>
									<th class="pb-1 pr-4 text-right">Last Analyze</th>
									<th class="pb-1 pr-4">Status</th>
								</tr>
							</thead>
							<tbody>
								{#each data.table_maintenance as t}
									<tr class="border-surface-secondary border-b last:border-0">
										<td class="text-primary py-1 pr-4 font-mono">{t.table_name}</td>
										<td class="text-secondary py-1 pr-4 text-right"
											>{formatNumber(t.live_tuples)}</td
										>
										<td class="text-secondary py-1 pr-4 text-right"
											>{formatNumber(t.dead_tuples)}</td
										>
										<td class="text-secondary py-1 pr-4 text-right"
											>{(t.dead_ratio * 100).toFixed(1)}%</td
										>
										<td class="text-secondary py-1 pr-4 text-right"
											>{formatDate(t.last_autovacuum)}</td
										>
										<td class="text-secondary py-1 pr-4 text-right"
											>{formatDate(t.last_autoanalyze)}</td
										>
										<td class="py-1 pr-4">
											<span
												class="rounded px-1.5 py-0.5 text-xs font-medium {statusBadge(t.status)}"
											>
												{t.status}
											</span>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</div>
			{/if}
		</section>

		<!-- Slow Queries -->
		<section class="border-surface-secondary rounded-md border">
			<button
				class="flex w-full items-center justify-between p-3 text-left hover:bg-surface-secondary/50"
				onclick={() => toggleSection('slow_queries')}
			>
				<h3 class="text-primary text-sm font-semibold">Slow Queries</h3>
				<div class="flex items-center gap-2">
					{#if expandedSections.slow_queries}
						<ChevronDown size={16} />
					{:else}
						<ChevronRight size={16} />
					{/if}
				</div>
			</button>
			{#if expandedSections.slow_queries}
				<div class="border-surface-secondary border-t p-3">
					{#if data.slow_queries == null}
						<p class="text-tertiary text-xs">Slow query data not available.</p>
					{:else if data.slow_queries.message}
						<p class="text-tertiary text-xs">{data.slow_queries.message}</p>
					{:else if data.slow_queries.queries.length === 0}
						<p class="text-tertiary text-xs">No slow queries found.</p>
					{:else}
						<div class="overflow-x-auto">
							<table class="w-full text-left text-xs">
								<thead>
									<tr class="text-tertiary border-surface-secondary border-b">
										<th class="pb-1 pr-4">Query</th>
										<th class="pb-1 pr-4 text-right">Calls</th>
										<th class="pb-1 pr-4 text-right">Total Time</th>
										<th class="pb-1 pr-4 text-right">Mean Time</th>
									</tr>
								</thead>
								<tbody>
									{#each data.slow_queries.queries as q}
										<tr class="border-surface-secondary border-b last:border-0">
											<td class="text-primary max-w-md truncate py-1 pr-4 font-mono">{q.query}</td>
											<td class="text-secondary py-1 pr-4 text-right">{formatNumber(q.calls)}</td>
											<td class="text-secondary py-1 pr-4 text-right"
												>{formatMs(q.total_exec_time_ms)}</td
											>
											<td class="text-secondary py-1 pr-4 text-right"
												>{formatMs(q.mean_exec_time_ms)}</td
											>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				</div>
			{/if}
		</section>

		<!-- Datatables -->
		<section class="border-surface-secondary rounded-md border">
			<button
				class="flex w-full items-center justify-between p-3 text-left hover:bg-surface-secondary/50"
				onclick={() => toggleSection('datatables')}
			>
				<h3 class="text-primary text-sm font-semibold">Datatables (Instance Storage)</h3>
				<div class="flex items-center gap-2">
					{#if expandedSections.datatables}
						<ChevronDown size={16} />
					{:else}
						<ChevronRight size={16} />
					{/if}
				</div>
			</button>
			{#if expandedSections.datatables}
				<div class="border-surface-secondary border-t p-3">
					{#if data.datatables.length === 0}
						<p class="text-tertiary text-xs">No instance-stored datatables found.</p>
					{:else}
						<div class="overflow-x-auto">
							<table class="w-full text-left text-xs">
								<thead>
									<tr class="text-tertiary border-surface-secondary border-b">
										<th class="pb-1 pr-4">Workspace</th>
										<th class="pb-1 pr-4">Name</th>
										<th class="pb-1 pr-4">Table</th>
										<th class="pb-1 pr-4 text-right">Size</th>
										<th class="pb-1 pr-4 text-right">Est. Rows</th>
									</tr>
								</thead>
								<tbody>
									{#each data.datatables as dt}
										<tr class="border-surface-secondary border-b last:border-0">
											<td class="text-secondary py-1 pr-4">{dt.workspace_id}</td>
											<td class="text-primary py-1 pr-4">{dt.name}</td>
											<td class="text-secondary py-1 pr-4 font-mono">{dt.table_name}</td>
											<td class="text-secondary py-1 pr-4 text-right">{dt.size_pretty}</td>
											<td class="text-secondary py-1 pr-4 text-right"
												>{formatNumber(Math.round(dt.estimated_rows))}</td
											>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				</div>
			{/if}
		</section>
	{:else if !loading}
		<p class="text-tertiary text-sm">
			Click "Run Diagnostics" to analyze your database health. The queries are read-only and
			lightweight.
		</p>
	{/if}
</div>
