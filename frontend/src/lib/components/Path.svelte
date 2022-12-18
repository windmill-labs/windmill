<script lang="ts">
	import { type Meta, pathToMeta } from '$lib/common'

	import {
		FlowService,
		ResourceService,
		ScheduleService,
		ScriptService,
		VariableService
	} from '$lib/gen'
	import { GroupService } from '$lib/gen'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { sleep } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Required from './Required.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { faEye, faPlus } from '@fortawesome/free-solid-svg-icons'
	import GroupEditor from './GroupEditor.svelte'
	import ToggleButtonGroup from './common/toggleButton/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton/ToggleButton.svelte'
	import { Icon } from 'svelte-awesome'
	import Tooltip from './Tooltip.svelte'

	type PathKind = 'resource' | 'script' | 'variable' | 'flow' | 'schedule' | 'app'
	let meta: Meta | undefined = undefined
	export let namePlaceholder = ''
	export let initialPath: string
	export let path = ''
	export let error = ''
	export let disabled = false

	export let kind: PathKind

	let inputP: HTMLInputElement | undefined = undefined

	const dispatch = createEventDispatcher()

	let groups: string[] = []

	$: meta && onMetaChange()

	function onMetaChange() {
		if (meta) {
			path = metaToPath(meta)
			validate(meta, path, kind)
		}
	}

	function metaToPath(meta: Meta): string {
		return [meta.ownerKind === 'group' ? 'g' : 'u', meta.owner, meta.name].join('/')
	}

	export function focus() {
		inputP?.focus()
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key

		if (key === 'Enter') {
			event.preventDefault()
			dispatch('enter')
		}
	}

	export async function reset() {
		if (path == '' || path == 'u//') {
			meta = { ownerKind: 'user', name: namePlaceholder, owner: '' }

			meta.owner = $userStore!.username

			let i = 1
			while (await pathExists(metaToPath(meta), kind)) {
				meta.name = `${namePlaceholder}_${i}`
				i += 1
			}
			path = metaToPath(meta)
		} else {
			meta = pathToMeta(path)
		}
	}

	async function loadGroups(): Promise<void> {
		let initialGroups: string[] = []
		if (initialPath?.split('/')?.[0] == 'g') {
			initialGroups.push(initialPath?.split('/')?.[1])
		}
		groups = initialGroups.concat(
			await GroupService.listGroupNames({
				workspace: $workspaceStore!,
				onlyMemberOf: !$userStore?.is_admin
			})
		)
	}

	async function validate(meta: Meta, path: string, kind: PathKind) {
		error = ''
		validateName(meta) && validatePath(path, kind)
	}

	let validateTimeout: NodeJS.Timeout | undefined = undefined

	async function validatePath(path: string, kind: PathKind): Promise<void> {
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if ((path == '' || path != initialPath) && (await pathExists(path, kind))) {
				error = 'path already used'
			} else if (meta && validateName(meta)) {
				error = ''
			}
			validateTimeout = undefined
		}, 500)
	}

	async function pathExists(path: string, kind: PathKind): Promise<boolean> {
		if (kind == 'flow') {
			return await FlowService.existsFlowByPath({ workspace: $workspaceStore!, path: path })
		} else if (kind == 'script') {
			return await ScriptService.existsScriptByPath({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'resource') {
			return await ResourceService.existsResource({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'variable') {
			return await VariableService.existsVariable({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'schedule') {
			return await ScheduleService.existsSchedule({ workspace: $workspaceStore!, path: path })
		} else {
			return false
		}
	}

	function validateName(meta: Meta): boolean {
		if (meta.name == undefined || meta.name == '') {
			error = 'Choose a name'
			return false
		} else if (!/^[\w-]+(\/[\w-]+)*$/.test(meta.name)) {
			error = 'This name is not valid'
			return false
		} else {
			return true
		}
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadGroups()
			initPath()
		}
	}

	function initPath() {
		if (initialPath == undefined || initialPath == '') {
			reset()
		} else {
			meta = pathToMeta(initialPath)
			onMetaChange()
			path = initialPath
		}
	}

	let newGroup: Drawer
	let viewGroup: Drawer
	let newGroupName: string
	let groupCreated: string | undefined = undefined

	async function addGroup() {
		await GroupService.createGroup({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newGroupName }
		})
		groupCreated = newGroupName
		loadGroups()
	}
</script>

<Drawer bind:this={newGroup}>
	<DrawerContent title="New Group" on:close={newGroup.closeDrawer}>
		<div class="flex flex-row">
			<input class="mr-2" placeholder="New group name" bind:value={newGroupName} />
			<Button size="md" startIcon={{ icon: faPlus }} disabled={!newGroupName} on:click={addGroup}>
				New&nbsp;group
			</Button>
		</div>

		{#if groupCreated}
			<div class="mt-8" />
			<GroupEditor name={groupCreated} />
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={viewGroup}>
	<DrawerContent title="Group {meta?.owner}" on:close={viewGroup.closeDrawer}>
		<GroupEditor name={meta?.owner ?? ''} />
	</DrawerContent>
</Drawer>

<div>
	<div class="flex flex-col sm:flex-row items-center gap-2 sm:gap-4 pb-0 mb-1">
		{#if meta != undefined}
			<div class="flex gap-4 shrink">
				<label class="block">
					<span class="text-gray-700 text-sm whitespace-nowrap">&nbsp;</span>

					<ToggleButtonGroup
						class="mt-0.5"
						bind:selected={meta.ownerKind}
						on:selected={(e) => {
							const kind = e.detail
							console.log(kind)
							if (meta) {
								if (kind === 'group') {
									meta.owner = 'all'
								} else {
									meta.owner = $userStore?.username ?? ''
								}
							}
						}}
					>
						<ToggleButton light size="xs" value="user" position="left">User</ToggleButton>
						<ToggleButton light size="xs" value="group" position="right">Group</ToggleButton>
					</ToggleButtonGroup>
				</label>
				{#if meta.ownerKind === 'user'}
					<label class="block shrink min-w-0">
						<span
							><span class="text-gray-700 text-sm mr-1">Owner</span><Tooltip
								>The prefix of a path defines the owner of an item. An owner has write permissions
								and can modify the path. An item can still be made writable or readable using
								granular perissioning.
								<a href="https://docs.windmill.dev/docs/reference/namespaces">See docs</a></Tooltip
							></span
						>
						<input
							class="!w-36"
							type="text"
							bind:value={meta.owner}
							placeholder={$userStore?.username ?? ''}
							disabled={!($superadmin || ($userStore?.is_admin ?? false))}
						/>
					</label>
				{:else}
					<label class="block grow w-48">
						<span
							><span class="text-gray-700 text-sm mr-1">Owner</span><Tooltip
								>The prefix of a path defines the owner of an item. An owner has write permissions
								and can modify the path. An item can still be made writable or readable using
								granular perissioning.
								<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a></Tooltip
							></span
						>

						<div class="flex flex-row gap-1 w-full">
							<select class="grow w-full" {disabled} bind:value={meta.owner}>
								{#each groups as g}
									<option>{g}</option>
								{/each}
							</select>
							<Button variant="border" size="xs" on:click={viewGroup.openDrawer}>
								<Icon scale={0.8} data={faEye} /></Button
							>
							<Button variant="border" size="xs" on:click={newGroup.openDrawer}>
								<Icon scale={0.8} data={faPlus} /></Button
							></div
						>
					</label>
				{/if}
			</div>
			<label class="block grow">
				<span class="text-gray-700 text-sm">
					Name
					<Required required={true} />
				</span>
				<input
					{disabled}
					type="text"
					id="path"
					autofocus
					bind:this={inputP}
					autocomplete="off"
					on:keyup={handleKeyUp}
					bind:value={meta.name}
					placeholder={namePlaceholder}
					class={error === ''
						? ''
						: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
				/>
			</label>
		{/if}
	</div>

	<div class="flex-row flex justify-between">
		<div><span class="font-mono text-sm">{path}</span></div>
		<div class="text-red-600 text-2xs">{error}</div>
	</div>
</div>

<style>
	input:disabled {
		background: rgba(200, 200, 200, 0.267);
	}
</style>
