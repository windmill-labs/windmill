<script lang="ts">
	// import { page } from '$app/stores'
	import { page } from '$app/state'
	import type { ActionKind } from '$lib/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import AuditLogDetails from '$lib/components/auditLogs/AuditLogDetails.svelte'
	import AuditLogsFilters from '$lib/components/auditLogs/AuditLogsFilters.svelte'
	import AuditLogsTable from '$lib/components/auditLogs/AuditLogsTable.svelte'
	import AuditLogMobileFilters from '$lib/components/auditLogs/AuditLogMobileFilters.svelte'
	import { Alert, DrawerContent } from '$lib/components/common'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'

	import type { AuditLog } from '$lib/gen'
	import { AuditService } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import AuditLogsTimeline from '$lib/components/auditLogs/AuditLogsTimeline.svelte'

	let username: string = $derived(page.url.searchParams.get('username') ?? 'all')
	let pageIndex: number | undefined = $derived(Number(page.url.searchParams.get('page')) || 0)
	let before: string | undefined = $derived(page.url.searchParams.get('before') ?? undefined)
	let hasMore: boolean = $state(false)
	let after: string | undefined = $derived(page.url.searchParams.get('after') ?? undefined)
	let perPage: number | undefined = $derived(Number(page.url.searchParams.get('perPage')) || 100)
	let operation: string = $derived(page.url.searchParams.get('operation') ?? 'all')
	let resource: string | undefined = $derived(page.url.searchParams.get('resource') ?? undefined)
	let scope: undefined | 'all_workspaces' | 'instance' = $derived(
		(page.url.searchParams.get('scope') ?? undefined) as undefined | 'all_workspaces' | 'instance'
	)

	let actionKind: ActionKind | 'all' = $derived(
		(page.url.searchParams.get('actionKind') as ActionKind) ?? 'all'
	)

	let logs: AuditLog[] = $state([])

	let selectedId: number | undefined = $state(undefined)
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

		console.log(
			`Fetching missing job span for ${jobId} from ${firstJobLog.username} around ${firstJobLog.timestamp}`
		)

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
					console.log(`Found job execution log with operation ${operation} for job ${jobId}`)
					return additionalLogs
				}
			}

			console.log(`No matching job execution found for job ${jobId}`)
			return []
		} catch (error) {
			console.error(`Failed to fetch missing job span for ${jobId}:`, error)
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
	<div class="w-full h-screen">
		<AuditLogsTimeline
			{logs}
			minTimeSet={after}
			maxTimeSet={before}
			onMissingJobSpan={fetchMissingJobSpan}
			onZoom={({ min, max }) => {
				before = max.toISOString()
				after = min.toISOString()
				console.log('zoom!')
			}}
			onLogSelected={(log) => {
				console.log('selected log ')
				selectedId = log.id
			}}
		/>
		<div class="px-2">
			<div class="flex items-center space-x-2 flex-row justify-between">
				<div class="flex flex-row flex-wrap justify-between py-2 my-4 px-4 gap-1 items-center">
					<h1 class="!text-2xl font-semibold leading-6 tracking-tight">Audit logs</h1>
					<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/audit_logs">
						You can only see your own audit logs unless you are an admin.
					</Tooltip>
				</div>
				<div class="hidden 2xl:block">
					<AuditLogsFilters
						bind:logs
						bind:username
						bind:before
						bind:after
						bind:actionKind
						bind:operation
						bind:resource
						bind:pageIndex
						bind:perPage
						bind:scope
						bind:hasMore
					/>
				</div>
				<div class="2xl:hidden">
					<AuditLogMobileFilters>
						{#snippet filters()}
							<AuditLogsFilters
								bind:logs
								bind:username
								bind:before
								bind:after
								bind:actionKind
								bind:operation
								bind:resource
								bind:scope
								bind:hasMore
							/>
						{/snippet}
					</AuditLogMobileFilters>
				</div>
			</div>

			{#if !$enterpriseLicense || $enterpriseLicense.endsWith('_pro')}
				<Alert title="Redacted audit logs" type="warning">
					You need an enterprise license to see unredacted audit logs.
				</Alert>
				<div class="py-2"></div>
			{/if}
		</div>
		<SplitPanesWrapper class="hidden md:block">
			<Splitpanes>
				<Pane size={70} minSize={50}>
					<AuditLogsTable
						{logs}
						{selectedId}
						bind:pageIndex
						bind:perPage
						bind:actionKind
						bind:operation
						bind:usernameFilter={username}
						bind:resourceFilter={resource}
						bind:hasMore
						on:select={(e) => {
							selectedId = e.detail
						}}
					/>
				</Pane>
				<Pane size={30} minSize={15}>
					{#if logs}
						<AuditLogDetails {logs} {selectedId} />
					{/if}
				</Pane>
			</Splitpanes>
		</SplitPanesWrapper>

		<div class="md:hidden">
			<AuditLogsTable
				{logs}
				bind:hasMore
				bind:pageIndex
				bind:perPage
				bind:actionKind
				bind:operation
				bind:usernameFilter={username}
				bind:resourceFilter={resource}
				on:select={(e) => {
					selectedId = e.detail

					auditLogDrawer?.openDrawer()
				}}
			/>
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
