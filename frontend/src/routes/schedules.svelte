<script context="module">
	export function load() {
		return {
			stuff: { title: 'Schedules' }
		}
	}
</script>

<script lang="ts">
	import { sendUserToast, displayDate, canWrite } from '$lib/utils'
	import { type Schedule, ScheduleService } from '$lib/gen'

	import PageHeader from '$lib/components/PageHeader.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import { goto } from '$app/navigation'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import {
		faEdit,
		faPlus,
		faShare,
		faToggleOff,
		faToggleOn,
		faTrash
	} from '@fortawesome/free-solid-svg-icons'
	import { userStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Badge, Button, Skeleton } from '$lib/components/common'

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
			sendUserToast(`Cannot ` + (enabled ? 'disable' : 'enable') + ` schedule: ${err}`, true)
			loadSchedules()
		}
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadSchedules()
		}
	}
</script>

<CenteredPage>
	<PageHeader title="Schedules" tooltip="Trigger Scripts and Flows according to a cron schedule">
		<Button size="md" startIcon={{ icon: faPlus }} href="/schedule/add">New&nbsp;schedule</Button>
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
					<th>Schedule</th>
					<th>Script or Flow</th>
					<th>schedule</th>
					<th>off/on</th>
					<th>timezone</th>
					<th>last edit</th>
					<th />
				</tr>
				<tbody slot="body">
					{#each schedules as { path, edited_by, edited_at, schedule, offset_, enabled, script_path, is_flow, extra_perms, canWrite }}
						<tr class={enabled ? '' : 'bg-gray-50'}>
							<td class="max-w-sm"
								><a class="break-words text-sm" href="/schedule/add?edit={path}&isFlow={is_flow}"
									>{path}</a
								>
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</td>
							<td
								><div class="inline-flex flex-row gap-x-2 align-middle"
									><Badge class="text-2xs font-mono ml-2">{is_flow ? 'flow' : 'script'}</Badge><a
										class="text-sm break-words"
										href="{is_flow ? '/flows/get' : '/scripts/get'}/{script_path}">{script_path}</a
									></div
								>
							</td><td><Badge color="blue">{schedule}</Badge></td>
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
								/></td
							>
							<td><Badge color="blue">{offset_ < 0 ? '+' : ''}{(offset_ / 60) * -1}</Badge></td>
							<td
								><span class="text-2xs">By {edited_by} <br />the {displayDate(edited_at)}</span></td
							>
							<td
								><Dropdown
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
												goto(`/schedule/add?edit=${path}&isFlow=${is_flow}`)
											}
										},
										{
											displayName: 'Share',
											icon: faShare,
											disabled: !canWrite,
											action: () => {
												shareModal.openDrawer(path)
											}
										}
									]}
									relative={true}
								/></td
							>
						</tr>
					{/each}
				</tbody>
			</TableCustom>
		{/if}
	</div>
</CenteredPage>

<ShareModal
	bind:this={shareModal}
	kind="schedule"
	on:change={() => {
		loadSchedules()
	}}
/>

<style>
	td {
		@apply px-2;
	}
</style>
