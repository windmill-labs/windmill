<script lang="ts">
	import { isOwner } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Path from './Path.svelte'
	import { AppService, FlowService, ScriptService } from '$lib/gen'

	const dispatch = createEventDispatcher()

	type Kind = 'script' | 'resource' | 'schedule' | 'variable' | 'flow' | 'app'

	let kind: Kind
	let initialPath: string = ''
	let path: string = ''

	let drawer: Drawer

	let own = false
	export async function openDrawer(initialPath_l: string, kind_l: Kind) {
		kind = kind_l
		initialPath = initialPath_l
		await loadOwner()
		drawer.openDrawer()
	}

	async function loadOwner() {
		own = await isOwner(path, $userStore!, $workspaceStore!)
	}

	async function updatePath() {
		if (kind == 'flow') {
			const flow = await FlowService.getFlowByPath({
				workspace: $workspaceStore!,
				path: initialPath
			})
			await FlowService.updateFlow({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					path,
					summary: flow.summary,
					description: flow.description,
					value: flow.value
				}
			})
		} else if (kind == 'script') {
			const script = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: initialPath
			})
			await ScriptService.createScript({
				workspace: $workspaceStore!,
				requestBody: {
					...script,
					description: script.description ?? '',
					lock: script.lock?.split('\n'),
					parent_hash: script.hash,
					path
				}
			})
		} else if (kind == 'app') {
			await AppService.updateApp({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					path
				}
			})
		}
		dispatch('update', path)
		drawer.closeDrawer()
	}
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Move {initialPath}" on:close={drawer.closeDrawer}>
		<div class="flex flex-col gap-6">
			<h1>Move {initialPath} to</h1>
			{#if !own}
				<Alert type="warning" title="Not owner"
					>Since you do not own this item, you cannot move this item (you can however fork it)</Alert
				>
			{/if}
			<Path disabled={!own} {kind} {initialPath} bind:path />
			<Button disabled={!own} on:click={updatePath}>Move</Button>
			<div />
		</div>
	</DrawerContent>
</Drawer>
