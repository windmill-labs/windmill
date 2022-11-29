<script lang="ts">
	import { FlowService, ScriptService, type Flow, type Script } from '$lib/gen'
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
		faShare,
		faWind
	} from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Button } from './common'
	import Dropdown from './Dropdown.svelte'
	import SharedBadge from './SharedBadge.svelte'
	import type ShareModal from './ShareModal.svelte'
	import Star from './Star.svelte'

	export let flow: Flow & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean

	export let shareModal: ShareModal
	const { summary, path, extra_perms, canWrite } = flow

	async function archiveFlow(path: string): Promise<void> {
		try {
			await FlowService.archiveFlowByPath({ workspace: $workspaceStore!, path })
			dispatch('change')
			sendUserToast(`Successfully archived flow ${path}`)
		} catch (err) {
			sendUserToast(`Could not archive this flow ${err.body}`, true)
		}
	}

	const dispatch = createEventDispatcher()
</script>

<a
	class="border border-gray-400  py-2 px-4  rounded-sm shadow-sm hover:border-blue-600 text-gray-800 flex flex-row items-center justify-between"
	href="/flows/get/{path}"
>
	<div class="flex flex-col gap-1 w-full h-full">
		<div class="font-semibold text-gray-700 truncate">
			<Icon data={faWind} class="mr-2" scale={1} />

			{#if marked}
				{@html marked}
			{:else}
				{!summary || summary.length == 0 ? path : summary}
			{/if}
		</div>
		<div class="flex flex-row  justify-between w-full grow gap-2 items-start">
			<div class="text-gray-700 text-xs flex flex-row  flex-wrap  gap-x-1 items-center"
				>{path}
				<Star kind="flow" {path} {starred} on:starred={() => dispatch('change')} />
				<SharedBadge {canWrite} extraPerms={extra_perms} />
			</div>
			<div class="flex flex-row-reverse place gap-x-2 pt-4">
				<div>
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
				</div>
				<div>
					<Button color="dark" size="xs" href="/flows/run/{path}" startIcon={{ icon: faPlay }}
						>Run</Button
					>
				</div>
				{#if canWrite}
					<div>
						<Button
							color="dark"
							variant="border"
							size="xs"
							href="/flows/edit/{path}"
							startIcon={{ icon: faEdit }}
						>
							Edit
						</Button>
					</div>
				{:else}
					<div>
						<Button
							color="dark"
							variant="border"
							size="xs"
							href="/flows/add?template={path}"
							startIcon={{ icon: faCodeFork }}
						>
							Fork
						</Button>
					</div>
				{/if}
			</div>
		</div></div
	></a
>
