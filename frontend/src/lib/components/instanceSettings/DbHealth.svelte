<script lang="ts">
	import { Button, Tab, Tabs } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, RefreshCw, ChevronDown, ChevronRight, ArrowDown } from 'lucide-svelte'

	let loading = $state(false)
	let jobsLoading = $state(false)
	let data: any = $state(null)
	let jobsData: any = $state(null)
	let error: string | null = $state(null)
	let jobsError: string | null = $state(null)
	let scanLimit = $state(10000)
	let activeTab = $state('overview')

	type SlowQuerySort = 'total' | 'mean' | 'calls'
	let slowSort: SlowQuerySort = $state('total')
	let slowSortLoading = $state(false)
	let expandedQueries: Record<number, boolean> = $state({})

	async function resetSlowStats() {
		if (
			!confirm(
				'Reset pg_stat_statements? This clears cumulative stats for ALL queries on this postgres instance.'
			)
		)
			return
		try {
			const response = await fetch(`/api/db_health/slow_queries/reset`, {
				method: 'POST',
				credentials: 'include'
			})
			if (!response.ok) {
				const text = await response.text()
				throw new Error(text || `HTTP ${response.status}`)
			}
			// refetch with current sort
			const refetch = await fetch(`/api/db_health/slow_queries?sort=${slowSort}`, {
				credentials: 'include'
			})
			if (refetch.ok && data) {
				data.slow_queries = await refetch.json()
				expandedQueries = {}
			}
			sendUserToast('pg_stat_statements reset successfully', false)
		} catch (e: any) {
			sendUserToast('Failed to reset stats: ' + e.message, true)
		}
	}

	async function setSlowSort(sort: SlowQuerySort) {
		if (sort === slowSort || slowSortLoading) return
		slowSort = sort
		slowSortLoading = true
		try {
			const response = await fetch(`/api/db_health/slow_queries?sort=${sort}`, {
				credentials: 'include'
			})
			if (!response.ok) {
				const text = await response.text()
				throw new Error(text || `HTTP ${response.status}`)
			}
			const slowQueries = await response.json()
			if (data) {
				data.slow_queries = slowQueries
				expandedQueries = {}
			}
		} catch (e: any) {
			sendUserToast('Failed to re-sort slow queries: ' + e.message, true)
		} finally {
			slowSortLoading = false
		}
	}

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

	async function runFastDiagnostics() {
		loading = true
		error = null
		try {
			const response = await fetch(`/api/db_health`, { credentials: 'include' })
			if (!response.ok) {
				const text = await response.text()
				throw new Error(text || `HTTP ${response.status}`)
			}
			data = await response.json()
			// main endpoint always returns slow_queries sorted by total
			slowSort = 'total'
			expandedQueries = {}
		} catch (e: any) {
			error = e.message
			sendUserToast('Failed to run diagnostics: ' + e.message, true)
		} finally {
			loading = false
		}
	}

	async function runJobsDiagnostics() {
		jobsLoading = true
		jobsError = null
		try {
			const response = await fetch(`/api/db_health/jobs?scan_limit=${scanLimit}`, {
				credentials: 'include'
			})
			if (!response.ok) {
				const text = await response.text()
				throw new Error(text || `HTTP ${response.status}`)
			}
			jobsData = await response.json()
		} catch (e: any) {
			jobsError = e.message
			sendUserToast('Failed to run job diagnostics: ' + e.message, true)
		} finally {
			jobsLoading = false
		}
	}

	$effect(() => {
		if (activeTab === 'overview') {
			runFastDiagnostics()
		} else if (activeTab === 'jobs') {
			runJobsDiagnostics()
		}
	})

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
	<Tabs bind:selected={activeTab}>
		<Tab value="overview" label="Overview" />
		<Tab value="jobs" label="Jobs" />
	</Tabs>

	{#if activeTab === 'overview'}
		<div class="flex items-center justify-end gap-2">
			<Button
				variant="subtle"
				onclick={runFastDiagnostics}
				disabled={loading}
				startIcon={{ icon: loading ? Loader2 : RefreshCw }}
				size="xs"
			>
				{loading ? 'Refreshing...' : 'Refresh'}
			</Button>
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

			<!-- Connection Pool -->
			<section class="border-surface-secondary rounded-md border">
				<button
					class="flex w-full items-center justify-between p-3 text-left hover:bg-surface-secondary/50"
					onclick={() => toggleSection('connection_pool')}
				>
					<h3 class="text-primary text-sm font-semibold">Database Connections</h3>
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
							Total connections: <strong>{data.connection_pool.pg_total_connections}</strong> / Max:
							<strong>{data.connection_pool.pg_max_connections}</strong>
						</p>
						<p class="text-secondary">
							Active: <strong>{data.connection_pool.pg_active_connections}</strong>
							/ Idle: <strong>{data.connection_pool.pg_idle_connections}</strong>
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
							<div class="flex flex-col gap-2 text-xs">
								<p class="text-secondary">{data.slow_queries.message}</p>
								<div class="bg-surface-secondary rounded p-2">
									<p class="text-secondary mb-1 font-semibold">
										Setup (requires superuser + a postgres restart):
									</p>
									<ol class="text-tertiary ml-4 list-decimal space-y-1">
										<li>
											On the postgres server, enable the preload library:
											<pre
												class="text-primary mt-0.5 overflow-x-auto whitespace-pre-wrap break-all font-mono"
												>ALTER SYSTEM SET shared_preload_libraries = 'pg_stat_statements';</pre
											>
										</li>
										<li>
											Restart postgres so the library is actually loaded (e.g. <code
												class="text-primary font-mono">sudo systemctl restart postgresql</code
											>, or use your managed DB console for RDS/Cloud SQL).
										</li>
										<li>
											On this database, create the extension:
											<pre
												class="text-primary mt-0.5 overflow-x-auto whitespace-pre-wrap break-all font-mono"
												>CREATE EXTENSION pg_stat_statements;</pre
											>
										</li>
										<li>Click Refresh on this page.</li>
									</ol>
								</div>
							</div>
						{:else}
							<div class="flex items-start justify-between gap-6 pb-3">
								<div class="text-tertiary flex flex-col gap-0.5 text-xs">
									<div class="flex items-center gap-2">
										<span
											>Top 50 queries from pg_stat_statements, sorted server-side. Click a column to
											re-sort. Click a row to show the full query.</span
										>
										{#if slowSortLoading}
											<Loader2 size={12} class="animate-spin shrink-0" />
										{/if}
									</div>
									{#if data.slow_queries.stats_reset}
										<span>Stats since: {formatDate(data.slow_queries.stats_reset)}</span>
									{/if}
								</div>
								<div class="shrink-0">
									<Button variant="subtle" size="xs" onclick={resetSlowStats}>Reset stats</Button>
								</div>
							</div>
							{#if data.slow_queries.queries.length === 0}
								<p class="text-tertiary text-xs">No slow queries found.</p>
							{:else}
								<div class="overflow-x-auto">
									<table class="w-full text-left text-xs">
										<thead>
											<tr class="text-tertiary border-surface-secondary border-b">
												<th class="pb-1 pr-4">Query</th>
												<th class="pb-1 pr-4 text-right">
													<button
														class="hover:text-primary inline-flex items-center gap-0.5"
														onclick={() => setSlowSort('calls')}
														disabled={slowSortLoading}
													>
														Calls
														{#if slowSort === 'calls'}
															<ArrowDown size={10} />
														{/if}
													</button>
												</th>
												<th class="pb-1 pr-4 text-right">
													<button
														class="hover:text-primary inline-flex items-center gap-0.5"
														onclick={() => setSlowSort('total')}
														disabled={slowSortLoading}
													>
														Total Time
														{#if slowSort === 'total'}
															<ArrowDown size={10} />
														{/if}
													</button>
												</th>
												<th class="pb-1 pr-4 text-right">
													<button
														class="hover:text-primary inline-flex items-center gap-0.5"
														onclick={() => setSlowSort('mean')}
														disabled={slowSortLoading}
													>
														Mean Time
														{#if slowSort === 'mean'}
															<ArrowDown size={10} />
														{/if}
													</button>
												</th>
											</tr>
										</thead>
										<tbody>
											{#each data.slow_queries.queries as q, i}
												<tr
													class="border-surface-secondary hover:bg-surface-secondary/40 cursor-pointer border-b last:border-0"
													onclick={() => (expandedQueries[i] = !expandedQueries[i])}
												>
													<td class="text-primary py-1 pr-4 font-mono">
														{#if expandedQueries[i]}
															<pre class="whitespace-pre-wrap break-all">{q.query}</pre>
														{:else}
															<div class="max-w-md truncate">{q.query}</div>
														{/if}
													</td>
													<td class="text-secondary py-1 pr-4 text-right align-top"
														>{formatNumber(q.calls)}</td
													>
													<td class="text-secondary py-1 pr-4 text-right align-top"
														>{formatMs(q.total_exec_time_ms)}</td
													>
													<td class="text-secondary py-1 pr-4 text-right align-top"
														>{formatMs(q.mean_exec_time_ms)}</td
													>
												</tr>
											{/each}
										</tbody>
									</table>
								</div>
							{/if}
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
		{/if}
	{/if}

	{#if activeTab === 'jobs'}
		<div class="flex items-center justify-between gap-2">
			<label class="text-tertiary flex items-center gap-1 whitespace-nowrap text-xs">
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
			<Button
				variant="subtle"
				onclick={runJobsDiagnostics}
				disabled={jobsLoading}
				startIcon={{ icon: jobsLoading ? Loader2 : RefreshCw }}
				size="xs"
			>
				{jobsLoading ? 'Scanning...' : 'Refresh'}
			</Button>
		</div>

		{#if jobsError}
			<div
				class="rounded border border-red-300 bg-red-50 p-3 text-sm text-red-700 dark:border-red-700 dark:bg-red-950 dark:text-red-300"
			>
				{jobsError}
			</div>
		{/if}
		<!-- Job Retention -->
		<section class="border-surface-secondary rounded-md border">
			<button
				class="flex w-full items-center justify-between p-3 text-left hover:bg-surface-secondary/50"
				onclick={() => toggleSection('job_retention')}
			>
				<h3 class="text-primary text-sm font-semibold">Job Retention</h3>
				<div class="flex items-center gap-2">
					{#if jobsData}
						<span
							class="rounded px-1.5 py-0.5 text-xs font-medium {statusBadge(
								jobsData.job_retention.status
							)}"
						>
							{jobsData.job_retention.status}
						</span>
					{:else if jobsLoading}
						<Loader2 size={14} class="text-tertiary animate-spin" />
					{/if}
					{#if expandedSections.job_retention}
						<ChevronDown size={16} />
					{:else}
						<ChevronRight size={16} />
					{/if}
				</div>
			</button>
			{#if expandedSections.job_retention}
				<div class="border-surface-secondary border-t p-3 text-xs">
					{#if jobsData}
						<div class="flex flex-col gap-1">
							<p class="text-secondary">
								Total completed jobs: <strong
									>{formatNumber(jobsData.job_retention.total_completed_jobs)}</strong
								>
							</p>
							<p class="text-secondary">
								Oldest job: <strong>{formatDate(jobsData.job_retention.oldest_completed_at)}</strong
								>
							</p>
							<p class="text-secondary">
								Retention period: <strong>
									{jobsData.job_retention.retention_period_secs
										? formatNumber(jobsData.job_retention.retention_period_secs) + 's'
										: 'Not configured'}
								</strong>
							</p>
							<p class="{statusColor(jobsData.job_retention.status)} mt-1 font-medium">
								{jobsData.job_retention.message}
							</p>
						</div>
					{:else}
						<p class="text-tertiary">Click Refresh to load.</p>
					{/if}
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
					{#if jobsData && jobsData.large_results.avg_result_size_bytes != null}
						<span class="text-tertiary text-xs"
							>avg: {formatBytes(jobsData.large_results.avg_result_size_bytes)}</span
						>
					{:else if jobsLoading}
						<Loader2 size={14} class="text-tertiary animate-spin" />
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
					{#if !jobsData}
						<p class="text-tertiary text-xs">Click Refresh to load.</p>
					{:else if jobsData.large_results.top_large_results.length === 0}
						<p class="text-tertiary text-xs"
							>No job results larger than 1 KB found in the scanned jobs.</p
						>
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
									{#each jobsData.large_results.top_large_results as r}
										<tr class="border-surface-secondary border-b last:border-0">
											<td class="text-primary py-1 pr-4 font-mono"
												><a
													href="/run/{r.id}?workspace={r.workspace_id}"
													class="text-blue-600 hover:underline dark:text-blue-400"
													>{r.id.substring(0, 8)}...</a
												></td
											>
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
	{/if}
</div>
