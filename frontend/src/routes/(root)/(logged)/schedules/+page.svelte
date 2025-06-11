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
		Calendar,
		Circle,
		Code,
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
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import JobPreview from '$lib/components/jobs/JobPreview.svelte'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { setQuery } from '$lib/navigation'
	import { onMount, untrack } from 'svelte'
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

	$effect(() => {
		if ($workspaceStore && $userStore) {
			untrack(() => {
				loadSchedules()
			})
		}
	})
	let scheduleEditor: ScheduleEditor | undefined = $state()

	let filteredItems: (ScheduleW & { marked?: any })[] | undefined = $state([])
	let items: typeof filteredItems | undefined = $state([])
	let filter = $state('')
	let ownerFilter: string | undefined = $state(undefined)
	let nbDisplayed = $state(15)

	let filterEnabledDisabled: 'all' | 'enabled' | 'disabled' = $state('all')

	const SCHEDULE_PATH_KIND_FILTER_SETTING = 'schedulePathKindFilter'
	const FILTER_USER_FOLDER_SETTING_NAME = 'user_and_folders_only'
	let selectedFilterKind = $state(
		(getLocalSetting(SCHEDULE_PATH_KIND_FILTER_SETTING) as 'schedule' | 'script_flow') ?? 'schedule'
	)
	let filterUserFolders = $state(getLocalSetting(FILTER_USER_FOLDER_SETTING_NAME) == 'true')

	$effect(() => {
		storeLocalSetting(SCHEDULE_PATH_KIND_FILTER_SETTING, selectedFilterKind)
	})
	$effect(() => {
		storeLocalSetting(FILTER_USER_FOLDER_SETTING_NAME, filterUserFolders ? 'true' : undefined)
	})

	function filterItemsPathsBaseOnUserFilters(
		item: ScheduleW,
		selectedFilterKind: 'schedule' | 'script_flow',
		filterUserFolders: boolean
	) {
		if ($workspaceStore == 'admins') return true
		if (filterUserFolders) {
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

	let preFilteredItems = $derived.by(() => {
		return ownerFilter != undefined
			? selectedFilterKind === 'schedule'
				? schedules?.filter(
						(x) =>
							x.path.startsWith(ownerFilter + '/') &&
							filterItemsPathsBaseOnUserFilters(x, selectedFilterKind, filterUserFolders) &&
							filterItemsBasedOnEnabledDisabled(x, filterEnabledDisabled)
					)
				: schedules?.filter(
						(x) =>
							x.script_path.startsWith(ownerFilter + '/') &&
							filterItemsPathsBaseOnUserFilters(x, selectedFilterKind, filterUserFolders) &&
							filterItemsBasedOnEnabledDisabled(x, filterEnabledDisabled)
					)
			: schedules?.filter(
					(x) =>
						filterItemsPathsBaseOnUserFilters(x, selectedFilterKind, filterUserFolders) &&
						filterItemsBasedOnEnabledDisabled(x, filterEnabledDisabled)
				)
	})

	$effect(() => {
		if ($workspaceStore) {
			ownerFilter = undefined
		}
	})

	let owners = $derived(
		selectedFilterKind === 'schedule'
			? Array.from(
					new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
				).sort()
			: Array.from(
					new Set(filteredItems?.map((x) => x.script_path.split('/').slice(0, 2).join('/')) ?? [])
				).sort()
	)

	$effect(() => {
		items = filter !== '' ? filteredItems : preFilteredItems
	})

	function updateQueryFilters(selectedFilterKind, filterUserFolders, filterEnabledDisabled) {
		setQuery(new URL(window.location.href), 'filter_kind', selectedFilterKind).then(() => {
			setQuery(
				new URL(window.location.href),
				'user_and_folders_only',
				String(filterUserFolders)
			).then(() => {
				setQuery(new URL(window.location.href), 'status', filterEnabledDisabled)
			})
		})
	}

	function loadQueryFilters() {
		let url = new URL(window.location.href)
		let queryFilterKind = url.searchParams.get('filter_kind')
		let queryFilterUserFolders = url.searchParams.get('user_and_folders_only')
		let queryFilterEnabledDisabled = url.searchParams.get('status')
		if (queryFilterKind) {
			selectedFilterKind = queryFilterKind as 'schedule' | 'script_flow'
		}
		if (queryFilterUserFolders) {
			filterUserFolders = queryFilterUserFolders == 'true'
		}
		if (queryFilterEnabledDisabled) {
			filterEnabledDisabled = queryFilterEnabledDisabled as 'all' | 'enabled' | 'disabled'
		}
	}

	onMount(() => {
		loadQueryFilters()
	})

	$effect(() => {
		updateQueryFilters(selectedFilterKind, filterUserFolders, filterEnabledDisabled)
	})
</script>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<ScheduleEditor onUpdate={loadSchedules} bind:this={scheduleEditor} />

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => (x.summary ?? '') + ' ' + x.path + ' (' + x.script_path + ')'}
/>

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
				size="md"
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
				<input type="text" placeholder="Search schedule" bind:value={filter} class="search-item" />
				<div class="flex flex-row items-center gap-2 mt-6">
					<div class="text-sm shrink-0"> Filter by path of </div>
					<ToggleButtonGroup bind:selected={selectedFilterKind}>
						{#snippet children({ item })}
							<ToggleButton small value="schedule" label="Schedule" icon={Calendar} {item} />
							<ToggleButton small value="script_flow" label="Script/Flow" icon={Code} {item} />
						{/snippet}
					</ToggleButtonGroup>
				</div>
				<ListFilters syncQuery bind:selectedFilter={ownerFilter} filters={owners} />

				<div class="flex flex-row items-center justify-end gap-4">
					<ToggleButtonGroup className="h-6 w-auto" bind:selected={filterEnabledDisabled}>
						{#snippet children({ item })}
							<ToggleButton small value="all" label="All" {item} />
							<ToggleButton small value="enabled" label="Enabled" {item} />
							<ToggleButton small value="disabled" label="Disabled" {item} />
						{/snippet}
					</ToggleButtonGroup>
					{#if $userStore?.is_super_admin && $userStore.username.includes('@')}
						<Toggle size="xs" bind:checked={filterUserFolders} options={{ right: 'Only f/*' }} />
					{:else if $userStore?.is_admin || $userStore?.is_super_admin}
						<Toggle
							size="xs"
							bind:checked={filterUserFolders}
							options={{ right: `Only u/${$userStore.username} and f/*` }}
						/>
					{/if}
				</div>
			</div>
			{#if loading}
				{#each new Array(6) as _}
					<Skeleton layout={[[6], 0.4]} />
				{/each}
			{:else if !schedules?.length}
				<div class="text-center text-sm text-tertiary mt-2"> No schedules </div>
			{:else if items?.length}
				<div class="border rounded-md divide-y">
					{#each items.slice(0, nbDisplayed) as { path, error, summary, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, extra_perms, canWrite, marked, jobs, paused_until } (path)}
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
									onclick={() => scheduleEditor?.openEdit(path, is_flow)}
									class="min-w-0 grow hover:underline decoration-gray-400"
								>
									<div class="text-primary flex-wrap text-left text-md font-semibold mb-1 truncate">
										{#if marked}
											<span class="text-xs">
												{@html marked}
											</span>
										{:else}
											{summary || script_path}
										{/if}
									</div>
									<div class="text-secondary text-xs truncate text-left font-light">
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
										startIcon={{ icon: canWrite ? Pen : Eye }}
										color="dark"
									>
										{canWrite ? 'Edit' : 'View'}
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
									<div class="flex gap-1 ml-0.5 text-[0.7em] text-tertiary items-center">
										<Loader2 size={14} class="animate-spin" />
										<span>Job stats loading...</span>
									</div>
								{:else}
									<div class="flex gap-1.5 ml-0.5 items-baseline flex-row-reverse">
										{#if avg_s}
											<div class="pl-2 text-tertiary text-2xs"
												>Avg: {(avg_s / 1000).toFixed(2)}s</div
											>
										{/if}
										{#each jobs ?? [] as job}
											{@const h = (avg_s ? job.duration_ms / avg_s : 1) * 7 + 3}
											<a href="{base}/run/{job.id}?workspace={$workspaceStore}">
												<JobPreview id={job.id}>
													<div>
														<div
															class="{job.success ? 'bg-green-300' : 'bg-red-300'} mx-auto w-1.5"
															style="height: {h}px"
														></div>
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
		{#if items && items?.length > 15 && nbDisplayed < items.length}
			<span class="text-xs"
				>{nbDisplayed} items out of {items.length}
				<button class="ml-4" onclick={() => (nbDisplayed += 30)}>load 30 more</button></span
			>
		{/if}
	</CenteredPage>
{/if}

<ShareModal
	bind:this={shareModal}
	on:change={() => {
		loadSchedules()
	}}
/>
