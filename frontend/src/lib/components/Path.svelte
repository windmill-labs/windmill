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
	import Tooltip from './Tooltip.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sleep } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'

	type PathKind = 'resource' | 'script' | 'variable' | 'flow' | 'schedule'
	export let meta: Meta = {
		ownerKind: 'user',
		owner: '',
		name: ''
	}
	export let namePlaceholder = ''
	export let initialPath: string
	export let path = ''
	export let error = ''

	export let kind: PathKind

	const dispatch = createEventDispatcher()

	let groups: Group[] = []

	$: path = metaToPath(meta)

	function metaToPath(meta: Meta): string {
		return [meta.ownerKind === 'group' ? 'g' : 'u', meta.owner, meta.name].join('/')
	}

	export function getPath() {
		return path
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
			meta.ownerKind = 'user'

			while ($userStore == undefined) {
				await sleep(500)
			}
			meta.owner = $userStore!.username
			meta.name = namePlaceholder
			let i = 1
			while (await pathExists(metaToPath(meta), kind)) {
				meta.name = `${namePlaceholder}_${i}`
				i += 1
			}
		} else {
			meta = pathToMeta(path)
		}
	}

	$: validate(meta, path, kind)

	async function loadGroups(): Promise<void> {
		groups = await GroupService.listGroups({ workspace: $workspaceStore! })
		meta.owner = meta.owner
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
			} else if (validateName(meta)) {
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
		}
	}

	$: {
		if (initialPath == undefined || initialPath == '') {
			reset()
		} else {
			meta = pathToMeta(initialPath)
		}
	}
</script>

<div>
	<div class="flex flex-col sm:grid sm:grid-cols-4 sm:gap-4 pb-0 mb-1">
		<label class="block">
			<span class="text-gray-700 text-sm whitespace-nowrap">
				Owner Kind <Tooltip>
					<slot name="ownerToolkit" />
				</Tooltip>
			</span>

			<select
				bind:value={meta.ownerKind}
				on:change={() => {
					if (meta.ownerKind === 'group') {
						meta.owner = 'all'
					} else {
						meta.owner = $userStore?.username ?? ''
					}
				}}
			>
				<option>user</option>
				<option>group</option>
			</select>
		</label>
		{#if meta.ownerKind === 'user'}
			<label class="block">
				<span class="text-sm text-gray-700">Owner</span>
				<input
					bind:value={meta.owner}
					placeholder={$userStore?.username ?? ''}
					disabled={!($userStore?.is_admin ?? false)}
				/>
			</label>
		{:else}
			<label class="block">
				<span class="text-sm text-gray-700">Owner</span>
				<select bind:value={meta.owner}>
					{#each groups as g}
						<option>{g.name}</option>
					{/each}
				</select>
			</label>
		{/if}
		<label class="block col-span-2">
			<span class="text-gray-700 text-sm">Name<span class="text-red-600 text-sm">*</span></span>
			<input
				autofocus
				autocomplete="off"
				on:keyup={handleKeyUp}
				bind:value={meta.name}
				placeholder={namePlaceholder}
				class={error === ''
					? ''
					: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
			/>
		</label>
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
