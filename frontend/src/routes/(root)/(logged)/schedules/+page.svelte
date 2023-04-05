<script lang="ts">
	import { ScheduleService, type Schedule } from '$lib/gen'
	import { canWrite, displayDate, sendUserToast } from '$lib/utils'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Badge, Button, Skeleton } from '$lib/components/common'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		faCircle,
		faEdit,
		faList,
		faPlus,
		faShare,
		faToggleOff,
		faToggleOn,
		faTrash
	} from '@fortawesome/free-solid-svg-icons'
	import { Icon } from 'svelte-awesome'

	type ScheduleW = Schedule & { canWrite: boolean }

	let schedules: ScheduleW[] = []
	let shareModal: ShareModal
	let loading = true

	async function loadSchedules(): Promise<void> {
		schedules = (await ScheduleService.listSchedules({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
		})
		loading = false
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

	$: {
		if ($workspaceStore && $userStore) {
			loadSchedules()
		}
	}
	let scheduleEditor: ScheduleEditor
</script>

<ScheduleEditor on:update={loadSchedules} bind:this={scheduleEditor} />
<CenteredPage>
	<PageHeader title="Schedules" tooltip="Trigger Scripts and Flows according to a cron schedule">
		<Button size="md" startIcon={{ icon: faPlus }} on:click={() => scheduleEditor.openNew(false)}>
			New&nbsp;schedule
		</Button>
	</PageHeader>
	<div class="mt-10 mb-40">
		{#if loading}
			<Skeleton layout={[0.5, [2.1], 0.7]} />
			{#each new Array(6) as _}
				<Skeleton layout={[[4], 0.7]} />
			{/each}
		{:else}
			<TableCustom>
				<tr slot="header-row">
					<th class="!px-0" />
					<th>Schedule</th>
					<th>Script/Flow</th>
					<th>Schedule</th>
					<th>Timezone</th>
					<th />
					<th>Enabled</th>
					<th>Last Edit</th>
					<th />
				</tr>
				<tbody slot="body">
					{#each schedules as { path, error, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, extra_perms, canWrite }}
						<tr class={enabled ? '' : 'bg-gray-50'}>
							<td class="!px-0 text-center">
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</td>
							<td class="max-w-sm">
								<button
									class="break-words text-left text-sm text-blue-600 font-normal"
									on:click={() => scheduleEditor?.openEdit(path, is_flow)}
								>
									{path}
								</button>
							</td>
							<td>
								<div class="inline-flex flex-row gap-x-2 align-middle w-full">
									<div class="grow">
										<a
											class="text-sm break-words"
											href="{is_flow ? '/flows/get' : '/scripts/get'}/{script_path}"
										>
											{script_path}
										</a>
									</div>
									<Badge class="text-2xs font-mono">{is_flow ? 'flow' : 'script'}></Badge>
								</div>
							</td>
							<td>
								<Badge color="blue">{schedule}</Badge>
							</td>
							<td>
								<Badge color="blue">{timezone}</Badge>
							</td>
							<td>
								<div class="w-10">
									{#if error}
										<Popover notClickable>
											<span class="flex h-4 w-4">
												<Icon
													class="text-red-600 animate-ping absolute inline-flex "
													data={faCircle}
													scale={0.7}
													label="Error during last job scheduling"
												/>
												<Icon
													class="text-red-600 relative inline-flex"
													data={faCircle}
													scale={0.7}
													label="Error during last job scheduling"
												/>
											</span>
											<div slot="text">
												The schedule disabled itself because there was an error scheduling the next
												job: {error}
											</div>
										</Popover>
									{/if}
								</div>
							</td>
							<td>
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
							</td>
							<td>
								<span class="text-2xs">By {edited_by} <br />the {displayDate(edited_at)}</span>
							</td>
							<td>
								<div class="inline-flex gap-2">
									<Button
										href={`/runs/${script_path}`}
										size="xs"
										startIcon={{ icon: faList }}
										color="light"
										variant="border"
									>
										Runs
									</Button>
									<Dropdown
										placement="bottom-end"
										dropdownItems={[
											{
												displayName: enabled ? 'Disable' : 'Enable',
												icon: enabled ? faToggleOff : faToggleOn,
												disabled: !canWrite,
												action: () => {
													setScheduleEnabled(path, enabled ? false : true)
												}
											},
											{
												displayName: 'Delete',
												type: 'delete',
												icon: faTrash,
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
												icon: faEdit,
												disabled: !canWrite,
												action: () => {
													scheduleEditor?.openEdit(path, is_flow)
												}
											},
											{
												displayName: 'View Runs',
												icon: faList,
												href: '/runs/' + path
											},
											{
												displayName: canWrite ? 'Share' : 'See Permissions',
												icon: faShare,
												action: () => {
													shareModal.openDrawer(path, 'schedule')
												}
											}
										]}
									/>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</TableCustom>
		{/if}
	</div>
</CenteredPage>

<ShareModal
	bind:this={shareModal}
	on:change={() => {
		loadSchedules()
	}}
/>

<style lang="postcss">
	td {
		@apply px-2;
	}
</style>
