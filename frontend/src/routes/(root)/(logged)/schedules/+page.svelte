<script lang="ts">
	import {
		ScheduleService,
		type ScheduleWJobs,
		type WorkspaceDeployUISettings,
		WorkspaceService
	} from '$lib/gen'
	import { canWrite, displayDate, getLocalSetting, storeLocalSetting } from '$lib/utils'
	import { base } from '$app/paths'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Badge, Button, Skeleton } from '$lib/components/common'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { userStore, workspaceStore, userWorkspaces, enterpriseLicense } from '$lib/stores'
	import {
		Circle,
		Copy,
		Eye,
		FileUp,
		List,
		Loader2,
		Pen,
		Play,
		Plus,
		Share,
		Trash
	} from 'lucide-svelte'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import FilterSearchbar, { useUrlSyncedFilterInstance } from '$lib/components/FilterSearchbar.svelte'
	import { buildSchedulesFilterSchema } from '$lib/components/schedules/schedulesFilter'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import JobPreview from '$lib/components/jobs/JobPreview.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { untrack } from 'svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { ALL_DEPLOYABLE, isDeployable } from '$lib/utils_deployable'
	import { runScheduleNow } from '$lib/components/triggers/scheduled/utils'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'

	type ScheduleW = ScheduleWJobs & { canWrite: boolean }

	let schedules: ScheduleW[] = $state([])
	let shareModal: ShareModal | undefined = $state()
	let loading = $state(true)
	let loadingSchedulesWithJobStats = $state(true)
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state()
	let deployUiSettings: WorkspaceDeployUISettings | undefined = $state(undefined)

	async function getDeployUiSettings() {
		if (!$enterpriseLicense) {
			deployUiSettings = ALL_DEPLOYABLE
			return
		}
		let settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		deployUiSettings = settings.deploy_ui ?? ALL_DEPLOYABLE
	}
	getDeployUiSettings()
	async function loadSchedules(): Promise<void> {
		const currentFilters = filters.val

		// Build API parameters from filters
		const apiParams: any = {
			workspace: $workspaceStore!
		}

		if (currentFilters.schedule_path) {
			apiParams.schedulePath = currentFilters.schedule_path
		}
		if (currentFilters.path_start) {
			apiParams.pathStart = currentFilters.path_start
		}
		if (currentFilters.path) {
			apiParams.path = currentFilters.path
		}
		if (currentFilters.description) {
			apiParams.description = currentFilters.description
		}
		if (currentFilters.summary) {
			apiParams.summary = currentFilters.summary
		}
		if (currentFilters.args) {
			apiParams.args = currentFilters.args
		}

		const result = (await ScheduleService.listSchedules(apiParams)).map((x) => {
			return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
		})

		// Extract unique values for autocomplete
		allPaths = Array.from(new Set(result.map((x) => x.path))).sort()
		allScriptPaths = Array.from(new Set(result.map((x) => x.script_path))).sort()

		schedules = result
		loading = false
		// after the schedule core data has been loaded, load all the job stats
		// TODO: we could potentially not reload the job stats on every call to loadSchedules, but for now it's
		// simpler to always call it. Update if performance becomes an issue.
		loadSchedulesWithJobStats()
	}

	// Reload schedules when filters change
	$effect(() => {
		filters.val
		if ($workspaceStore) {
			untrack(() => loadSchedules())
		}
	})

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

	$effect(() => {
		if ($workspaceStore && $userStore) {
			untrack(() => {
				loadSchedules()
			})
		}
	})
	let scheduleEditor: ScheduleEditor | undefined = $state()

	// Collect unique values for filter autocomplete
	let allPaths: string[] = $state([])
	let allScriptPaths: string[] = $state([])

	// FilterSearchbar setup
	let userFoldersFilterType = $derived(
		$userStore?.is_super_admin && $userStore.username.includes('@')
			? 'only f/*'
			: $userStore?.is_admin || $userStore?.is_super_admin
				? 'u/username and f/*'
				: undefined
	)
	let schedulesFilterSchema = $derived(
		buildSchedulesFilterSchema({
			paths: allPaths,
			scriptPaths: allScriptPaths,
			showUserFoldersFilter: userFoldersFilterType !== undefined,
			userFoldersLabel:
				userFoldersFilterType === 'only f/*'
					? 'Only f/*'
					: `Only u/${$userStore?.username} and f/*`
		})
	)
	let filters = useUrlSyncedFilterInstance(untrack(() => schedulesFilterSchema))

	let nbDisplayed = $state(15)
	let filterEnabledDisabled: 'all' | 'enabled' | 'disabled' = $state('all')

	const SCHEDULE_PATH_KIND_FILTER_SETTING = 'schedulePathKindFilter'
	let selectedFilterKind = $state(
		(getLocalSetting(SCHEDULE_PATH_KIND_FILTER_SETTING) as 'schedule' | 'script_flow') ?? 'schedule'
	)

	$effect(() => {
		storeLocalSetting(SCHEDULE_PATH_KIND_FILTER_SETTING, selectedFilterKind)
	})

	function filterItemsPathsBaseOnUserFilters(
		item: ScheduleW,
		selectedFilterKind: 'schedule' | 'script_flow',
		userFoldersOnly: boolean
	) {
		if ($workspaceStore == 'admins') return true
		if (userFoldersOnly) {
			if (selectedFilterKind === 'schedule') {
				return (
					!item.path.startsWith('u/') || item.path.startsWith('u/' + $userStore?.username + '/')
				)
			} else {
				return (
					!item.script_path.startsWith('u/') ||
					item.script_path.startsWith('u/' + $userStore?.username + '/')
				)
			}
		} else {
			return true
		}
	}

	function filterItemsBasedOnEnabledDisabled(
		item: ScheduleW,
		filterEnabledDisabled: 'all' | 'enabled' | 'disabled'
	) {
		if (filterEnabledDisabled === 'all') return true
		if (filterEnabledDisabled === 'enabled') return item.enabled
		if (filterEnabledDisabled === 'disabled') return !item.enabled
	}

	// Filter schedules client-side for enabled/disabled and user folders
	let filteredItems = $derived.by(() => {
		return schedules?.filter(
			(x) =>
				filterItemsPathsBaseOnUserFilters(x, selectedFilterKind, !!filters.val.user_folders_only) &&
				filterItemsBasedOnEnabledDisabled(x, filterEnabledDisabled)
		)
	})

	let items = $derived(filteredItems)
</script>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<ScheduleEditor onUpdate={loadSchedules} bind:this={scheduleEditor} />

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.schedules}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title="Schedules"
			tooltip="Trigger Scripts and Flows according to a cron schedule"
			documentationLink="https://www.windmill.dev/docs/core_concepts/scheduling"
		>
			<Button
				size="lg"
				variant="accent"
				startIcon={{ icon: Plus }}
				on:click={() => scheduleEditor?.openNew(false)}
				aiId="schedules-add-schedule"
				aiDescription="Add schedule"
			>
				New schedule
			</Button>
		</PageHeader>
		<div class="w-full h-full flex flex-col">
			<div class="w-full pb-4 pt-6">
				<FilterSearchbar schema={schedulesFilterSchema} bind:value={filters.val} />

				<div class="flex flex-row items-center justify-end gap-4 mt-4">
					<ToggleButtonGroup class="w-auto" bind:selected={filterEnabledDisabled}>
						{#snippet children({ item })}
							<ToggleButton small value="all" label="All" {item} />
							<ToggleButton small value="enabled" label="Enabled" {item} />
							<ToggleButton small value="disabled" label="Disabled" {item} />
						{/snippet}
					</ToggleButtonGroup>
				</div>
			</div>
			{#if loading}
				{#each new Array(6) as _}
					<Skeleton layout={[[6], 0.4]} />
				{/each}
			{:else if !schedules?.length}
				<div class="text-center text-xs font-semibold text-emphasis mt-2"> No schedules </div>
			{:else if items?.length}
				<div class="border rounded-md divide-y">
					{#each items.slice(0, nbDisplayed) as { path, error, summary, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, extra_perms, canWrite, jobs, paused_until } (path)}
						{@const href = `${is_flow ? '/flows/get' : '/scripts/get'}/${script_path}`}
						{@const avg_s = jobs
							? jobs.reduce((acc, x) => acc + x.duration_ms, 0) / jobs.length
							: undefined}

						<div
							class="bg-surface-tertiary hover:bg-surface-hover w-full items-center px-4 py-2 gap-4 first-of-type:!border-t-0
				first-of-type:rounded-t-md last-of-type:rounded-b-md flex flex-col"
						>
							<div class="w-full flex gap-4 items-center">
								<RowIcon kind={is_flow ? 'flow' : 'script'} />

								<a
									href="#{path}"
									onclick={() => scheduleEditor?.openEdit(path, is_flow)}
									class="min-w-0 grow hover:underline decoration-gray-400"
								>
									<div
										class="text-emphasis flex-wrap text-left text-xs font-semibold mb-1 truncate"
									>
										{summary || script_path}
									</div>
									<div class="text-secondary text-xs truncate text-left">
										schedule: {path}
									</div>
								</a>

								{#if paused_until && new Date(paused_until) > new Date()}
									<div class="pb-1">
										<Badge color="yellow"
											>Paused until {new Date(paused_until).toLocaleString()}</Badge
										>
									</div>
								{/if}

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
											{#snippet text()}
												<div>
													The schedule disabled itself because there was an error scheduling the
													next job: {error}
												</div>
											{/snippet}
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
										href={`${base}/runs/?schedule_path=${path}&show_schedules=true&show_future_jobs=true`}
										unifiedSize="md"
										startIcon={{ icon: List }}
										variant="subtle"
									>
										Runs
									</Button>
									<Button
										on:click={() => scheduleEditor?.openEdit(path, is_flow)}
										size="xs"
										startIcon={{ icon: canWrite ? Pen : Eye }}
										variant="subtle"
									>
										{canWrite ? 'Edit' : 'View'}
									</Button>
									<Dropdown
										size="md"
										items={[
											{
												displayName: `View ${is_flow ? 'Flow' : 'Script'}`,
												icon: Eye,
												action: () => {
													goto(href)
												}
											},
											{
												displayName: `Duplicate schedule`,
												icon: Copy,
												action: () => {
													scheduleEditor?.openNew(is_flow, script_path, path)
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
												displayName: canWrite ? 'Edit' : 'View',
												icon: canWrite ? Pen : Eye,
												action: () => {
													scheduleEditor?.openEdit(path, is_flow)
												}
											},
											...(isDeployable('trigger', path, deployUiSettings)
												? [
														{
															displayName: 'Deploy to prod/staging',
															icon: FileUp,
															action: () => {
																deploymentDrawer?.openDrawer(path, 'trigger', {
																	triggers: {
																		kind: 'schedules'
																	}
																})
															}
														}
													]
												: []),
											{
												displayName: 'View runs',
												icon: List,
												href:
													base +
													'/runs/?schedule_path=' +
													path +
													'&show_schedules=true&show_future_jobs=true'
											},
											{
												displayName: 'Audit logs',
												icon: Eye,
												href: `${base}/audit_logs?resource=${path}`
											},
											{
												displayName: 'Run now',
												icon: Play,
												action: () => {
													runScheduleNow(script_path, path, is_flow, $workspaceStore!)
												}
											},
											{
												displayName: canWrite ? 'Share' : 'See Permissions',
												icon: Share,
												action: () => {
													shareModal?.openDrawer(path, 'schedule')
												}
											}
										]}
									/>
								</div>
							</div>
							<div class="w-full flex justify-between items-baseline">
								{#if loadingSchedulesWithJobStats}
									<div class="flex gap-1 ml-0.5 text-xs text-secondary items-center">
										<Loader2 size={14} class="animate-spin" />
										<span>Job stats loading...</span>
									</div>
								{:else}
									<div class="flex gap-1.5 ml-0.5 items-baseline flex-row-reverse">
										{#if avg_s}
											<div class="pl-2 text-secondary text-xs"
												>Avg: {(avg_s / 1000).toFixed(2)}s</div
											>
										{/if}
										{#each jobs ?? [] as job}
											{@const h = (avg_s ? job.duration_ms / avg_s : 1) * 7 + 3}
											<a href="{base}/run/{job.id}?workspace={$workspaceStore}">
												<JobPreview id={job.id} class="p-4">
													<div>
														<div
															class="{job.success ? 'bg-green-300' : 'bg-red-300'} mx-auto w-1.5"
															style="height: {h}px"
														></div>
														<!-- <div class="text-[0.6em] mt-0.5 text-center text-primary"
														>{(job.duration_ms / 1000).toFixed(2)}s</div
													> -->
													</div>
												</JobPreview>
											</a>
										{/each}
									</div>
								{/if}
								<div
									class="flex flex-wrap text-xs text-secondary gap-1 items-center justify-end truncate pr-2"
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
		{#if items && items?.length > 15 && nbDisplayed < items.length}
			<div class="flex items-center gap-4 text-xs font-semibold text-emphasis">
				<span>{nbDisplayed} items out of {items.length}</span>
				<Button size="xs" variant="subtle" on:click={() => (nbDisplayed += 30)}>
					Load 30 more
				</Button>
			</div>
		{/if}
	</CenteredPage>
{/if}

<ShareModal
	bind:this={shareModal}
	on:change={() => {
		loadSchedules()
	}}
/>
