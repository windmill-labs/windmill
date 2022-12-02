<script lang="ts">
	import { goto } from '$app/navigation'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import Star from '$lib/components/Star.svelte'

	import { FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import {
		faArchive,
		faCalendarAlt,
		faCodeFork,
		faEdit,
		faEye,
		faList,
		faPlay,
		faShare
	} from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import { Wind } from 'svelte-lucide'

	import Button from '../button/Button.svelte'

	export let flow: Flow & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal

	let { summary, path, extra_perms, canWrite, workspace_id } = flow

	const dispatch = createEventDispatcher()

	async function archiveFlow(path: string): Promise<void> {
		try {
			await FlowService.archiveFlowByPath({ workspace: $workspaceStore!, path })
			dispatch('change')
			sendUserToast(`Successfully archived flow ${path}`)
		} catch (err) {
			sendUserToast(`Could not archive this flow ${err.body}`, true)
		}
	}
</script>

<tr class="hover:bg-gray-50 cursor-pointer" on:click={() => goto(`/flows/get/${path}`)}>
	<td class="pl-4 py-4 pr-1 w-8">
		<div
			class="bg-[#f0fdfa] rounded-md p-1 flex justify-center items-center border-[#99f6e4] border"
		>
			<Wind size="18px" color="#14b8a6" />
		</div>
	</td>

	<td class="px-2 py-4">
		<div class="text-gray-900 max-w-md flex-wrap text-md font-semibold mb-1">
			{#if marked}
				{@html marked}
			{:else}
				{!summary || summary.length == 0 ? path : summary}
			{/if}
		</div>
		<div class="text-gray-600 text-xs ">
			{path}
		</div>
	</td>
	<td class="px-2 py-4 w-64">
		<div class="flex flex-row max-w-xs gap-1 items-start flex-wrap">
			<SharedBadge {canWrite} extraPerms={extra_perms} />
		</div>
	</td>
	<td class="py-4 text-left text-sm font-semibold text-gray-900 px-2 w-0">
		<Star
			kind="script"
			{path}
			{starred}
			workspace_id={workspace_id ?? $workspaceStore ?? ''}
			on:starred={() => {
				dispatch('change')
			}}
		/>
	</td>

	<td class="py-4 pl-2 pr-6">
		<div class="w-full flex gap-1 items-center justify-end">
			<Dropdown
				dropdownItems={[
					{
						displayName: 'View flow',
						icon: faEye,
						href: `/flows/get/${path}`
					},
					{
						displayName: 'Edit',
						icon: faEdit,
						href: `/flows/edit/${path}`,
						disabled: !canWrite
					},
					{
						displayName: 'Use as template/Fork',
						icon: faCodeFork,
						href: `/flows/add?template=${path}`
					},
					{
						displayName: 'View runs',
						icon: faList,
						href: `/runs/${path}`
					},
					{
						displayName: 'Schedule',
						icon: faCalendarAlt,
						href: `/schedule/add?path=${path}&isFlow=true`
					},
					{
						displayName: 'Share',
						icon: faShare,
						action: () => {
							shareModal.openDrawer(path)
						},
						disabled: !canWrite
					},
					{
						displayName: 'Archive',
						icon: faArchive,
						action: () => {
							path ? archiveFlow(path) : null
						},
						type: 'delete',
						disabled: !canWrite
					}
				]}
			/>

			{#if canWrite}
				<div>
					<Button
						color="light"
						size="xs"
						variant="border"
						startIcon={{ icon: faEdit }}
						href="/flows/edit/{path}"
					>
						Edit
					</Button>
				</div>
			{:else}
				<div>
					<Button
						color="light"
						size="xs"
						variant="border"
						startIcon={{ icon: faCodeFork }}
						href="/flows/add?template={path}"
					>
						Fork
					</Button>
				</div>
			{/if}

			<Button
				href="/flows/get/{path}"
				color="light"
				variant="border"
				size="xs"
				spacingSize="md"
				startIcon={{ icon: faEye }}
			>
				Detail
			</Button>
			<Button
				href="/flows/run/{path}"
				color="dark"
				size="xs"
				spacingSize="md"
				endIcon={{ icon: faPlay }}
			>
				Run
			</Button>
		</div>
	</td>
</tr>
