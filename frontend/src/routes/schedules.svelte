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
	import Icon from 'svelte-awesome'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Button } from '$lib/components/common'

	type ScheduleW = Schedule & { canWrite: boolean }

	let schedules: ScheduleW[] = []

	let shareModal: ShareModal

	async function loadSchedules(): Promise<void> {
		schedules = (await ScheduleService.listSchedules({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
		})
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
			sendUserToast(`Cannot ` + enabled ? 'disable' : 'enable' + ` schedule: ${err}`, true)
		}
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadSchedules()
		}
	}
</script>

<CenteredPage>
  <PageHeader title="Schedule">
		<Button size="sm" startIcon={{ icon: faPlus }} href="/schedule/add">New&nbsp;schedule</Button>
	</PageHeader>
	<div class="relative mt-10">
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
					<tr class={enabled ? '' : 'bg-gray-100'}>
						<td class="max-w-sm"
							><a class="break-all" href="/schedule/add?edit={path}&isFlow={is_flow}">{path}</a>
							<SharedBadge {canWrite} extraPerms={extra_perms} />
						</td>
						<td class="whitespace-nowrap"
							><a href="{is_flow ? '/flows/get' : '/scripts/get'}/{script_path}">{script_path}</a
							><span class="text-2xs text-gray-500 bg-gray-100 font-mono ml-2"
								>{is_flow ? 'flow' : 'script'}</span
							></td
						>
						<td>{schedule}</td>
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
						<td>{offset_ < 0 ? '+' : ''}{(offset_ / 60) * -1}</td>
						<td class="text-2xs">By {edited_by} <br />at {displayDate(edited_at)}</td>
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
											shareModal.openModal(path)
										}
									}
								]}
								relative={false}
							/></td
						>
					</tr>
				{/each}
			</tbody>
		</TableCustom>
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
