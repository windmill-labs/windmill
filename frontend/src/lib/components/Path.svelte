<script lang="ts">
	import { type Meta, pathToMeta } from '$lib/common'

	import type { Group } from '$lib/gen'
	import { GroupService } from '$lib/gen'
	import Tooltip from './Tooltip.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sleep } from '$lib/utils'

	export let meta: Meta = {
		ownerKind: 'user',
		owner: '',
		name: ''
	}
	export let namePlaceholder = ''
	export let initialPath: string
	export let path = ''

	let groups: Group[] = []
	let error = ''

	$: {
		path = [meta.ownerKind === 'group' ? 'g' : 'u', meta.owner, meta.name].join('/')
	}

	export function getPath() {
		return path
	}
	export async function reset() {
		if (path == '' || path == 'u//') {
			meta.ownerKind = 'user'

			while ($userStore == undefined) {
				await sleep(500)
			}
			meta.owner = $userStore!.username
			meta.name = ''
		} else {
			meta = pathToMeta(path)
		}
	}

	$: validateName(meta)

	async function loadGroups(): Promise<void> {
		groups = await GroupService.listGroups({ workspace: $workspaceStore! })
		meta.owner = meta.owner
	}

	function validateName(meta: Meta): void {
		if (meta.name == undefined || meta.name == '') {
			error = 'choose a name'
			return
		}
		const regex = new RegExp(/^[\w-]+(\/[\w-]+)*$/)
		if (regex.test(meta.name)) {
			error = ''
		} else {
			error = 'This name is not valid. '
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
			<span class="text-gray-700 text-sm">
				Owner Kind<Tooltip class="mx-1">
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
				bind:value={meta.name}
				placeholder={namePlaceholder}
				class={error === ''
					? ''
					: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
			/>
		</label>
	</div>
	<div class="pt-0 text-xs px-1 flex flex-col-reverse sm:grid sm:grid-cols-4 sm:gap-4 w-full">
		<div class="col-span-2">Path: <span class="font-mono">{path}</span></div>
		<div class="text-purple-500 text-2xs col-span-2">{error}</div>
	</div>
</div>

<style>
	input:disabled {
		background: rgba(200, 200, 200, 0.267);
	}
</style>
