<script lang="ts">
	import { sendUserToast, truncateHash, displayDate, canWrite, getUser } from '../utils';
	import { type Schedule, ScheduleService } from '../gen';

	import PageHeader from './components/PageHeader.svelte';
	import TableCustom from './components/TableCustom.svelte';
	import Dropdown from './components/Dropdown.svelte';
	import { goto } from '$app/navigation';
	import ShareModal from './components/ShareModal.svelte';
	import SharedBadge from './components/SharedBadge.svelte';
	import {
		faEdit,
		faPlus,
		faShare,
		faToggleOff,
		faToggleOn
	} from '@fortawesome/free-solid-svg-icons';
	import { workspaceStore } from '../stores';
	import CenteredPage from './components/CenteredPage.svelte';
	import Icon from 'svelte-awesome';

	type ScheduleW = Schedule & { canWrite: boolean };

	let schedules: ScheduleW[] = [];

	let shareModal: ShareModal;

	async function loadSchedules(): Promise<void> {
		const user = await getUser($workspaceStore!);
		schedules = (await ScheduleService.listSchedules({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.path, x.extra_perms!, user), ...x };
		});
	}

	async function setScheduleEnabled(path: string, enabled: boolean): Promise<void> {
		try {
			await ScheduleService.setScheduleEnabled({
				path,
				workspace: $workspaceStore!,
				requestBody: { enabled }
			});
			loadSchedules();
		} catch (err) {
			sendUserToast(`Cannot ` + enabled ? 'disable' : 'enable' + ` schedule: ${err}`, true);
		}
	}

	$: {
		if ($workspaceStore) {
			loadSchedules();
		}
	}
</script>

<CenteredPage>
	<PageHeader title="Schedule">
		<button class="default-button" on:click={() => goto('/schedule/add')}
			><Icon class="text-white mb-1" data={faPlus} scale={0.9} /> &nbsp; New schedule</button
		>
	</PageHeader>
	<div class="relative mt-10">
		<TableCustom>
			<tr slot="header-row">
				<th>Schedule</th>
				<th>Script or Flow</th>
				<th>schedule</th>
				<th>on/off</th>
				<th>timezone</th>
				<th>last edit</th>
				<th />
			</tr>
			<tbody slot="body">
				{#each schedules as { path, edited_by, edited_at, schedule, offset_, enabled, script_path, is_flow, extra_perms, canWrite }}
					<tr class={enabled ? '' : 'bg-gray-200'}>
						<td
							><a href="/schedule/add?edit={path}&isFlow={is_flow}" style="cursor: pointer;"
								>{path}</a
							>
							<div>
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</div>
						</td>
						<td
							>{script_path}<span class="text-2xs text-gray-500 bg-gray-100 font-mono ml-2"
								>{is_flow ? 'flow' : 'script'}</span
							></td
						>
						<td>{schedule}</td>
						<td
							><a
								id="toggle-{path}"
								style="cursor: pointer;"
								on:click={() => {
									if (canWrite) {
										setScheduleEnabled(path, enabled ? false : true);
									} else {
										sendUserToast('not enough permission', true);
									}
								}}
								><span class="m-auto block text-center">
									<Icon data={enabled ? faToggleOn : faToggleOff} scale={1.5} />
								</span></a
							></td
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
											setScheduleEnabled(path, enabled ? false : true);
										}
									},
									{
										displayName: 'Edit',
										icon: faEdit,
										disabled: !canWrite,
										action: () => {
											goto(`/schedule/add?edit=${path}&isFlow=${is_flow}`);
										}
									},
									{
										displayName: 'Share',
										icon: faShare,
										disabled: !canWrite,
										action: () => {
											shareModal.openModal(path);
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
		loadSchedules();
	}}
/>

<style>
</style>
