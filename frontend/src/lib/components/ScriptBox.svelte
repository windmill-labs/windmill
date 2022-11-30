<script lang="ts">
	import { ScriptService, type Script } from '$lib/gen'
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
		faScroll,
		faShare
	} from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Badge, Button } from './common'
	import { LanguageIcon } from './common/languageIcons'
	import Dropdown from './Dropdown.svelte'
	import SharedBadge from './SharedBadge.svelte'
	import type ShareModal from './ShareModal.svelte'
	import Star from './Star.svelte'

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

<a
	class="border border-gray-400 py-2 px-4 rounded-sm shadow-sm  hover:border-blue-600 text-gray-800"
	href="/scripts/get/{hash}"
>
	<div class="flex flex-col gap-1 w-full h-full">
		<div class="font-semibold text-gray-700 truncate">
			<Icon data={faScroll} class="mr-2" scale={1} />

			{#if marked}
				{@html marked}
			{:else}
				{!summary || summary.length == 0 ? path : summary}
			{/if}
		</div>
		<div class="flex flex-row  justify-between w-full grow gap-2 items-start">
			<div class="text-gray-700 text-xs flex flex-row  flex-wrap  gap-x-1 items-center">
				{path}
				<Star
					kind="script"
					{path}
					{starred}
					workspace_id={workspace_id ?? $workspaceStore ?? ''}
					on:starred={() => dispatch('change')}
				/>
				<SharedBadge {canWrite} extraPerms={extra_perms} />
				<div><LanguageIcon height={16} lang={language} /></div>
				{#if kind != 'script'}
					<Badge color="blue" capitalize>{kind}</Badge>
				{/if}
				{#if lock_error_logs}
					<Badge color="red">Deployment error</Badge>
				{/if}
			</div>
			<div class="flex flex-col items-end grow pt-4">
				<div class="flex flex-row-reverse place gap-x-2">
					<div>
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
										shareModal.openDrawer(path)
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
					</div>
					<div>
						<Button color="dark" size="xs" startIcon={{ icon: faPlay }} href="/scripts/run/{hash}">
							Run
						</Button>
					</div>
					{#if canWrite}
						<div>
							<Button
								variant="border"
								color="dark"
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
								color="dark"
								variant="border"
								size="xs"
								startIcon={{ icon: faCodeFork }}
								href="/scripts/add?template={path}"
							>
								Fork
							</Button>
						</div>
					{/if}
				</div>
			</div></div
		>
	</div></a
>
