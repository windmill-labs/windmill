<script lang="ts">
	import { type Meta, pathToMeta } from '$lib/common'

	import {
		FlowService,
		ResourceService,
		ScheduleService,
		ScriptService,
		VariableService,
		type Group
	} from '$lib/gen'
	import { GroupService } from '$lib/gen'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { sleep } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Required from './Required.svelte'
	import Popover from './Popover.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import GroupEditor from './GroupEditor.svelte'

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

	let groups: Group[] = []

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

			while ($userStore == undefined) {
				await sleep(500)
			}
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
		groups = await GroupService.listGroups({ workspace: $workspaceStore! })
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
		if ($workspaceStore) {
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

<div>
	<div class="flex flex-col sm:grid sm:grid-cols-4 sm:gap-4 pb-0 mb-1">
		{#if meta != undefined}
			<label class="block">
				<span class="text-gray-700 text-sm whitespace-nowrap">
					<Popover
						>Owner Kind
						<span slot="text"
							>Select the group <span class="font-mono">all</span>
							to share it with all workspace users, and <span class="font-mono">user</span> to keep
							it private.
							<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
						</span>
					</Popover>
				</span>

				<select
					{disabled}
					bind:value={meta.ownerKind}
					on:change={() => {
						if (meta) {
							if (meta.ownerKind === 'group') {
								meta.owner = 'all'
							} else {
								meta.owner = $userStore?.username ?? ''
							}
						}
					}}
				>
					<option>user</option>
					<option>group</option>
				</select>
			</label>
			{#if meta.ownerKind === 'user'}
				<label class="block">
					<span class="text-gray-700 text-sm">Owner</span>
					<input
						type="text"
						bind:value={meta.owner}
						placeholder={$userStore?.username ?? ''}
						disabled={!($superadmin || ($userStore?.is_admin ?? false))}
					/>
				</label>
			{:else}
				<label class="block">
					<span class="text-gray-700 text-sm inline-flex justify-between w-full"
						>Owner <button class=" text-xs text-blue-500" on:click={newGroup.openDrawer}
							>+group</button
						></span
					>
					<select {disabled} bind:value={meta.owner}>
						{#each groups as g}
							<option>{g.name}</option>
						{/each}
					</select>
				</label>
			{/if}
			<label class="block col-span-2">
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

	<div class="pt-0 text-xs px-1 flex flex-col-reverse sm:grid sm:grid-cols-4 sm:gap-4 w-full">
		<div class="col-span-2"><span class="font-mono">{path}</span></div>
		<div class="text-red-600 text-2xs col-span-2">{error}</div>
	</div>
</div>

<style>
	input:disabled {
		background: rgba(200, 200, 200, 0.267);
	}
</style>
