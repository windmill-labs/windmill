<script lang="ts">
	import { page } from '$app/state'
	import type { ActionKind } from '$lib/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import AuditLogDetails from '$lib/components/auditLogs/AuditLogDetails.svelte'
	import AuditLogsFilters from '$lib/components/auditLogs/AuditLogsFilters.svelte'
	import AuditLogsTable from '$lib/components/auditLogs/AuditLogsTable.svelte'
	import AuditLogMobileFilters from '$lib/components/auditLogs/AuditLogMobileFilters.svelte'
	import { Alert, DrawerContent, Skeleton } from '$lib/components/common'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import AnimatedPane from '$lib/components/splitPanes/AnimatedPane.svelte'

	import type { AuditLog } from '$lib/gen'
	import { AuditService } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import AuditLogsTimeline from '$lib/components/auditLogs/AuditLogsTimeline.svelte'
	import { useAuditLogsLoader } from '$lib/components/auditLogs/useAuditLogsLoader.svelte'

	let username: string = $state(page.url.searchParams.get('username') ?? 'all')
	let before: string | undefined = $state(page.url.searchParams.get('before') ?? undefined)
	let after: string | undefined = $state(page.url.searchParams.get('after') ?? undefined)
	let perPage: number | undefined = $state(Number(page.url.searchParams.get('perPage')) || 100)
	let operation: string = $state(page.url.searchParams.get('operation') ?? 'all')
	let resource: string | undefined = $state(page.url.searchParams.get('resource') ?? undefined)
	let scope: undefined | 'all_workspaces' | 'instance' = $state(
		(page.url.searchParams.get('scope') ?? undefined) as undefined | 'all_workspaces' | 'instance'
	)

	let actionKind: ActionKind | 'all' = $state(
		(page.url.searchParams.get('actionKind') as ActionKind) ?? 'all'
	)

	let auditLogsLoader = useAuditLogsLoader(() => ({
		workspace: $workspaceStore,
		scope,
		username,
		operation,
		resource,
		actionKind,
		before,
		after,
		perPage: perPage ?? 100
	}))
	let logs: AuditLog[] | undefined = $derived(auditLogsLoader.logs)
	let batchProgress = $derived(auditLogsLoader.batchProgress)

	// Regrouping the timeline can fire extra requests to fill in missing job spans, so it gets the
	// result of a batched load once it settles rather than every intermediate batch.
	let timelineLogs: AuditLog[] | undefined = $state()
	$effect(() => {
		const settledLogs = batchProgress ? undefined : auditLogsLoader.logs
		if (settledLogs) {
			timelineLogs = settledLogs
		}
	})

	let selectedId: number | undefined = $state(undefined)
	// A selection the current rows no longer contain (filters or page changed) closes the pane.
	let detailOpen = $derived(
		selectedId !== undefined && !!logs?.some((log) => log.id === selectedId)
	)
	let auditLogDrawer: Drawer | undefined = $state()

	// Function to fetch missing job execution audit logs
	async function fetchMissingJobSpan(jobId: string, jobLogs: AuditLog[]): Promise<AuditLog[]> {
		if (jobLogs.length === 0) return []

		const firstJobLog = jobLogs[0]
		const timeBuffer = 10000 // 10 seconds buffer for safety

		// Create time range around the job execution
		const jobTime = new Date(firstJobLog.timestamp).getTime()
		const beforeTime = new Date(jobTime + timeBuffer).toISOString()
		const afterTime = new Date(jobTime - timeBuffer).toISOString()

		try {
			// Try multiple operation patterns to find the job execution
			const operationPatterns = ['jobs.run', 'jobs.run.script', 'jobs.run.flow', 'jobs.run.preview']

			for (const operation of operationPatterns) {
				const additionalLogs = await AuditService.listAuditLogs({
					workspace: scope === 'instance' ? 'global' : $workspaceStore!,
					username: firstJobLog.username,
					operation: operation,
					before: beforeTime,
					after: afterTime,
					perPage: 100,
					allWorkspaces: scope === 'all_workspaces'
				})

				// Check if we found the job execution log
				const jobExecutionLog = additionalLogs.find((log) => log.parameters?.uuid === jobId)
				if (jobExecutionLog) {
					return additionalLogs
				}
			}

			return []
		} catch (error) {
			return []
		}
	}
</script>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.audit_logs}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<div class="flex flex-col w-full h-screen">
		<div class="flex items-center space-x-2 flex-row justify-between">
			<div class="flex flex-row flex-wrap justify-between py-2 my-4 px-4 gap-1 items-center">
				<h1 class="text-2xl font-semibold text-emphasis">Audit logs</h1>
				<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/audit_logs">
					You can only see your own audit logs unless you are an admin.
				</Tooltip>
			</div>
			<div class="flex flex-row flex-wrap justify-between py-2 my-2 px-4 gap-1 items-center">
				<div class="hidden 2xl:block">
					<AuditLogsFilters
						{logs}
						bind:username
						bind:before
						bind:after
						bind:actionKind
						bind:operation
						bind:resource
						bind:perPage
						bind:scope
						loading={auditLogsLoader.loading}
						onRefresh={() => auditLogsLoader.reload()}
					/>
				</div>
				<div class="2xl:hidden">
					<AuditLogMobileFilters>
						{#snippet filters()}
							<AuditLogsFilters
								{logs}
								bind:username
								bind:before
								bind:after
								bind:actionKind
								bind:operation
								bind:resource
								bind:scope
								loading={auditLogsLoader.loading}
								onRefresh={() => auditLogsLoader.reload()}
							/>
						{/snippet}
					</AuditLogMobileFilters>
				</div>
			</div>
		</div>
		{#if !$enterpriseLicense || $enterpriseLicense.endsWith('_pro')}
			<div class="mx-4 mb-2">
				<Alert title="Redacted audit logs" type="warning">
					You need an enterprise license to see unredacted audit logs.
				</Alert>
			</div>
		{/if}

		<div class="h-2/6 shrink-0 p-2 px-4 bg-surface-tertiary mx-4 border rounded-md">
			{#if timelineLogs}
				<AuditLogsTimeline
					logs={timelineLogs}
					minTimeSet={after}
					maxTimeSet={before}
					onZoom={({ min, max }) => {
						before = max.toISOString()
						after = min.toISOString()
					}}
					onMissingJobSpan={fetchMissingJobSpan}
					onLogSelected={(log) => {
						selectedId = log.id
					}}
				/>
			{/if}
		</div>

		<div
			class="hidden md:block grow min-h-0 [&_.splitpanes\_\_splitter]:!bg-transparent [&_.splitpanes\_\_splitter]:!border-none"
		>
			<Splitpanes>
				<Pane minSize={40}>
					<div class="h-full flex flex-col p-4 pr-2">
						<div class="grow min-h-0 overflow-y-hidden overflow-x-auto">
							<!-- Also while a batched load has yet to return its first rows: the table footer
							     carries the progress row and its Stop button. -->
							{#if logs || batchProgress}
								<AuditLogsTable
									loading={auditLogsLoader.loading}
									{logs}
									{selectedId}
									bind:perPage
									bind:actionKind
									bind:operation
									bind:usernameFilter={username}
									bind:resourceFilter={resource}
									hasMore={auditLogsLoader.hasMore}
									loadingExtra={auditLogsLoader.loadingExtra}
									onLoadMore={() => auditLogsLoader.loadMore()}
									{batchProgress}
									batchSize={auditLogsLoader.currentBatchSize}
									onBatchSizeChange={(size) => auditLogsLoader.restreamWithBatchSize(size)}
									onStopLoading={() => auditLogsLoader.stopBatchLoading()}
									showWorkspace={scope === 'instance' || scope === 'all_workspaces'}
									onselect={(id) => {
										selectedId = selectedId === id ? undefined : id
									}}
								/>
							{:else}
								<div class="gap-1 flex flex-col p-2">
									{#each new Array(8) as _}
										<Skeleton layout={[[3]]} />
									{/each}
								</div>
							{/if}
						</div>
					</div>
				</Pane>
				<AnimatedPane size={40} minSize={15} opened={detailOpen}>
					<div class="h-full flex flex-col p-4 pl-2">
						<div class="flex-1 min-h-0 mt-8 overflow-y-auto border rounded-md bg-surface-tertiary">
							{#if logs}
								<AuditLogDetails
									{logs}
									{selectedId}
									onClose={() => {
										selectedId = undefined
									}}
								/>
							{/if}
						</div>
					</div>
				</AnimatedPane>
			</Splitpanes>
		</div>

		<div class="md:hidden grow min-h-0 flex flex-col p-4">
			<div class="grow min-h-0 overflow-y-hidden overflow-x-auto">
				<AuditLogsTable
					{logs}
					loading={auditLogsLoader.loading}
					hasMore={auditLogsLoader.hasMore}
					loadingExtra={auditLogsLoader.loadingExtra}
					onLoadMore={() => auditLogsLoader.loadMore()}
					bind:perPage
					bind:actionKind
					bind:operation
					bind:usernameFilter={username}
					bind:resourceFilter={resource}
					{batchProgress}
					batchSize={auditLogsLoader.currentBatchSize}
					onBatchSizeChange={(size) => auditLogsLoader.restreamWithBatchSize(size)}
					onStopLoading={() => auditLogsLoader.stopBatchLoading()}
					showWorkspace={scope === 'instance' || scope === 'all_workspaces'}
					onselect={(id) => {
						selectedId = id
						auditLogDrawer?.openDrawer()
					}}
				/>
			</div>
		</div>
	</div>
{/if}

<Drawer bind:this={auditLogDrawer}>
	<DrawerContent title="Log details" on:close={auditLogDrawer.closeDrawer}>
		{#if logs}
			<AuditLogDetails {logs} {selectedId} />
		{/if}
	</DrawerContent>
</Drawer>
