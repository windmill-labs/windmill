<script lang="ts">
	import { ScheduleService, JobService, type ScriptArgs, type ScheduleWJobs } from '$lib/gen'
	import { canWrite, displayDate } from '$lib/utils'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Badge, Button, Skeleton } from '$lib/components/common'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Circle, Eye, List, Loader2, Pen, Play, Plus, Share, Trash } from 'lucide-svelte'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import JobPreview from '$lib/components/jobs/JobPreview.svelte'

	type ScheduleW = ScheduleWJobs & { canWrite: boolean }

	let schedules: ScheduleW[] = []
	let shareModal: ShareModal
	let loading = true
	let loadingSchedulesWithJobStats = true

	async function loadSchedules(): Promise<void> {
		schedules = (await ScheduleService.listSchedules({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
		})
		loading = false
		// after the schedule core data has been loaded, load all the job stats
		// TODO: we could potentially not reload the job stats on every call to loadSchedules, but for now it's
		// simpler to always call it. Update if performance becomes an issue.
		loadSchedulesWithJobStats()
	}

	async function loadSchedulesWithJobStats(): Promise<void> {
		loadingSchedulesWithJobStats = true
		let schedulesWithJobsByPath = new Map<string, ScheduleW>()
		let schedulesWithJobsList = await ScheduleService.listSchedulesWithJobs({
			workspace: $workspaceStore!
		})
		schedulesWithJobsList.map((x) => {
			schedulesWithJobsByPath[x.path] = x
		})
		for (let schedule of schedules) {
			if (schedulesWithJobsByPath[schedule.path]) {
				schedule.jobs = schedulesWithJobsByPath[schedule.path].jobs
			}
		}
		loadingSchedulesWithJobStats = false
	}

	async function setScheduleEnabled(path: string, enabled: boolean): Promise<void> {
		try {
			await ScheduleService.setScheduleEnabled({
				path,
				workspace: $workspaceStore!,
				requestBody: { enabled }
			})
			loadSchedules()
		} catch (err) {
			sendUserToast(`Cannot ` + (enabled ? 'enable' : 'disable') + ` schedule: ${err.body}`, true)
			loadSchedules()
		}
	}

	async function runScheduleNow(
		path: string,
		args: ScriptArgs | undefined,
		isFlow: boolean
	): Promise<void> {
		try {
			const runByPath = isFlow ? JobService.runFlowByPath : JobService.runScriptByPath

			const run = await runByPath({
				path,
				requestBody: args ?? {},
				workspace: $workspaceStore!
			})

			sendUserToast(`Schedule ${path} will run now`, false, [
				{
					label: 'Go to the run page',
					callback: () => goto('/run/' + run + '?workspace=' + $workspaceStore)
				}
			])
		} catch (err) {
			sendUserToast(`Cannot run schedule now: ${err.body}`, true)
		}
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadSchedules()
		}
	}
	let scheduleEditor: ScheduleEditor

	let filteredItems: (ScheduleW & { marked?: any })[] | undefined = []
	let filter = ''
</script>

<ScheduleEditor on:update={loadSchedules} bind:this={scheduleEditor} />

<SearchItems
	{filter}
	items={schedules}
	bind:filteredItems
	f={(x) => x.path + ' ' + x.script_path}
/>

<CenteredPage>
	<PageHeader
		title="Schedules"
		tooltip="Trigger Scripts and Flows according to a cron schedule"
		documentationLink="https://www.windmill.dev/docs/core_concepts/scheduling"
	>
		<Button size="md" startIcon={{ icon: Plus }} on:click={() => scheduleEditor.openNew(false)}>
			New&nbsp;schedule
		</Button>
	</PageHeader>
	<div class="w-full h-full flex flex-col">
		<div class="w-12/12 pb-4 pt-6">
			<input type="text" placeholder="Search schedule" bind:value={filter} class="search-item" />
		</div>
		{#if loading}
			{#each new Array(6) as _}
				<Skeleton layout={[[6], 0.4]} />
			{/each}
		{:else if !schedules?.length}
			<div class="text-center text-sm text-tertiary mt-2"> No schedules </div>
		{:else if filteredItems?.length}
			<div class="border rounded-md divide-y">
				{#each filteredItems as { path, error, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, extra_perms, canWrite, args, marked, jobs }}
					{@const href = `${is_flow ? '/flows/get' : '/scripts/get'}/${script_path}`}
					{@const avg_s = jobs
						? jobs.reduce((acc, x) => acc + x.duration_ms, 0) / jobs.length
						: undefined}

					<div
						class="hover:bg-surface-hover w-full items-center px-4 py-2 gap-4 first-of-type:!border-t-0
				first-of-type:rounded-t-md last-of-type:rounded-b-md flex flex-col"
					>
						<div class="w-full flex gap-5 items-center">
							<RowIcon kind={is_flow ? 'flow' : 'script'} />

							<a
								href="#{path}"
								on:click={() => scheduleEditor?.openEdit(path, is_flow)}
								class="min-w-0 grow hover:underline decoration-gray-400"
							>
								<div class="text-primary flex-wrap text-left text-md font-semibold mb-1 truncate">
									{#if marked}
										<span class="text-xs">
											{@html marked}
										</span>
									{:else}
										{script_path}
									{/if}
								</div>
								<div class="text-secondary text-xs truncate text-left font-light">
									schedule: {path}
								</div>
							</a>

							<div class="gap-2 items-center hidden md:flex">
								<Badge large color="blue">{schedule}</Badge>
								<Badge small color="gray">{timezone}</Badge>
							</div>

							<div class="hidden lg:flex flex-row gap-1 items-center">
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</div>

							<div class="w-10">
								{#if error}
									<Popover notClickable>
										<span class="flex h-4 w-4">
											<Circle
												class="text-red-600 animate-ping absolute inline-flex fill-current"
												size={12}
											/>
											<Circle class="text-red-600 relative inline-flex fill-current" size={12} />
										</span>
										<div slot="text">
											The schedule disabled itself because there was an error scheduling the next
											job: {error}
										</div>
									</Popover>
								{/if}
							</div>

							<Toggle
								checked={enabled}
								on:change={(e) => {
									if (canWrite) {
										setScheduleEnabled(path, e.detail)
									} else {
										sendUserToast('not enough permission', true)
									}
								}}
							/>
							<div class="flex gap-2 items-center justify-end">
								<Button
									href={`/runs/?schedule_path=${path}`}
									size="xs"
									startIcon={{ icon: List }}
									color="light"
									variant="border"
								>
									Runs
								</Button>
								<Button
									on:click={() => scheduleEditor?.openEdit(path, is_flow)}
									size="xs"
									startIcon={{ icon: Pen }}
									color="dark"
								>
									Edit
								</Button>
								<Dropdown
									items={[
										{
											displayName: `View ${is_flow ? 'Flow' : 'Script'}`,
											icon: Eye,
											action: () => {
												goto(href)
											}
										},
										{
											displayName: 'Delete',
											type: 'delete',
											icon: Trash,
											disabled: !canWrite,
											action: async () => {
												await ScheduleService.deleteSchedule({
													workspace: $workspaceStore ?? '',
													path
												})
												loadSchedules()
											}
										},
										{
											displayName: 'Edit',
											icon: Pen,
											disabled: !canWrite,
											action: () => {
												scheduleEditor?.openEdit(path, is_flow)
											}
										},
										{
											displayName: 'View Runs',
											icon: List,
											href: '/runs/?schedule_path=' + path
										},
										{
											displayName: 'Run now',
											icon: Play,
											action: () => {
												runScheduleNow(script_path, args, is_flow)
											}
										},
										{
											displayName: canWrite ? 'Share' : 'See Permissions',
											icon: Share,
											action: () => {
												shareModal.openDrawer(path, 'schedule')
											}
										}
									]}
								/>
							</div>
						</div>
						<div class="w-full flex justify-between items-baseline">
							{#if loadingSchedulesWithJobStats}
								<div class="flex gap-1 ml-0.5 text-[0.7em] text-tertiary items-center">
									<Loader2 size={14} class="animate-spin" />
									<span>Job stats loading...</span>
								</div>
							{:else}
								<div class="flex gap-1.5 ml-0.5 items-baseline flex-row-reverse">
									{#if avg_s}
										<div class="pl-2 text-tertiary text-2xs">Avg: {(avg_s / 1000).toFixed(2)}s</div>
									{/if}
									{#each jobs ?? [] as job}
										{@const h = (avg_s ? job.duration_ms / avg_s : 1) * 7 + 3}
										<a href="/run/{job.id}?workspace={$workspaceStore}">
											<JobPreview id={job.id}>
												<div>
													<div
														class="{job.success ? 'bg-green-300' : 'bg-red-300'} mx-auto w-1.5"
														style="height: {h}px"
													/>
													<!-- <div class="text-[0.6em] mt-0.5 text-center text-tertiary"
														>{(job.duration_ms / 1000).toFixed(2)}s</div
													> -->
												</div>
											</JobPreview>
										</a>
									{/each}
								</div>
							{/if}
							<div
								class="flex flex-wrap text-[0.7em] text-tertiary gap-1 items-center justify-end truncate pr-2"
								><div class="truncate">edited by {edited_by}</div><div class="truncate"
									>the {displayDate(edited_at)}</div
								></div
							></div
						>
					</div>
				{/each}
			</div>
		{:else}
			<NoItemFound />
		{/if}
	</div>
</CenteredPage>

<ShareModal
	bind:this={shareModal}
	on:change={() => {
		loadSchedules()
	}}
/>
