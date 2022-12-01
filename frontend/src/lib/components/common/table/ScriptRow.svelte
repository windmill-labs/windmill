<script lang="ts">
	import Dropdown from '$lib/components/Dropdown.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import Star from '$lib/components/Star.svelte'

	import { ScriptService, type Script } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { capitalize, sendUserToast } from '$lib/utils'
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
	import { Code2 } from 'svelte-lucide'

	import Badge from '../badge/Badge.svelte'
	import Button from '../button/Button.svelte'

	export let script: Script & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal

	let {
		summary,
		path,
		hash,
		language,
		extra_perms,
		canWrite,
		lock_error_logs,
		kind,
		workspace_id
	} = script

	const dispatch = createEventDispatcher()

	async function archiveScript(path: string): Promise<void> {
		await ScriptService.archiveScriptByPath({ workspace: $workspaceStore!, path })
		dispatch('change')
		sendUserToast(`Successfully archived script ${path}`)
	}
</script>

<tr class="hover:bg-gray-50 cursor-pointer">
	<td class="pl-4 py-4 pr-1">
		<div class="bg-blue-50 rounded-md p-1 flex justify-center items-center border-blue-200 border">
			<Code2 size="18px" color="#60A5FA" />
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
	<td class="px-2 py-4">
		<div class="flex flex-row max-w-xs gap-1 items-start flex-wrap">
			<SharedBadge {canWrite} extraPerms={extra_perms} />
			{#if lock_error_logs}
				<Badge color="red">Deployment failed</Badge>
			{/if}
			<span
				class="inline-flex rounded-full bg-indigo-100 px-2 text-xs font-semibold leading-5 text-indigo-800"
			>
				{capitalize(language)}
			</span>
		</div>
	</td>
	<td class="py-4 text-left text-sm font-semibold text-gray-900 pl-6">
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
						displayName: 'View script',
						icon: faEye,
						href: `/scripts/get/${hash}`
					},
					{
						displayName: 'Edit',
						icon: faEdit,
						href: `/scripts/edit/${hash}`,
						disabled: !canWrite
					},
					{
						displayName: 'Edit code',
						icon: faEdit,
						href: `/scripts/edit/${hash}?step=2`,
						disabled: !canWrite
					},
					{
						displayName: 'Use as template',
						icon: faCodeFork,
						href: `/scripts/add?template=${path}`
					},
					{
						displayName: 'View runs',
						icon: faList,
						href: `/runs/${path}`
					},
					{
						displayName: 'Schedule',
						icon: faCalendarAlt,
						href: `/schedule/add?path=${path}`
					},
					{
						displayName: 'Share',
						icon: faShare,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(path)
						},
						disabled: !canWrite
					},
					{
						displayName: 'Archive',
						icon: faArchive,
						action: () => {
							path ? archiveScript(path) : null
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
						startIcon={{ icon: faEdit }}
						href="/scripts/edit/{hash}?step=2"
					>
						Edit
					</Button>
				</div>
			{:else}
				<div>
					<Button
						color="light"
						size="xs"
						startIcon={{ icon: faCodeFork }}
						href="/scripts/add?template={path}"
					>
						Fork
					</Button>
				</div>
			{/if}

			<Button
				href="/scripts/get/{hash}"
				color="light"
				variant="border"
				size="xs"
				spacingSize="md"
				startIcon={{ icon: faEye }}
			>
				Detail
			</Button>
			<Button
				href="/scripts/run/{hash}"
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
