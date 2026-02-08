<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Path from './Path.svelte'
	import { AppService, FlowService, ScriptService } from '$lib/gen'
	import { isOwner } from '$lib/utils'

	const dispatch = createEventDispatcher()

	type Kind = 'script' | 'resource' | 'schedule' | 'variable' | 'flow' | 'app'

	let kind: Kind
	let initialPath: string = ''
	let path: string | undefined = undefined
	let summary: undefined | string = undefined

	let drawer: Drawer

	let own = false
	export async function openDrawer(
		initialPath_l: string,
		summary_l: string | undefined,
		kind_l: Kind
	) {
		kind = kind_l
		path = undefined
		initialPath = initialPath_l
		summary = summary_l
		loadOwner()
		drawer.openDrawer()
	}

	function loadOwner() {
		own = isOwner(initialPath, $userStore!, $workspaceStore!)
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
					path: path ?? '',
					summary: summary ?? '',
					description: flow.description,
					value: flow.value,
					schema: flow.schema,
					tag: flow.tag,
					dedicated_worker: flow.dedicated_worker,
					ws_error_handler_muted: flow.ws_error_handler_muted,
					visible_to_runner_only: flow.visible_to_runner_only,
					on_behalf_of_email: flow.on_behalf_of_email
				}
			})
		} else if (kind == 'script') {
			const script = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: initialPath
			})
			script.summary = summary ?? ''
			await ScriptService.createScript({
				workspace: $workspaceStore!,
				requestBody: {
					...script,
					description: script.description ?? '',
					lock: script.lock,
					parent_hash: script.hash,
					path: path ?? ''
				}
			})
		} else if (kind == 'app') {
			await AppService.updateApp({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					path: path != initialPath ? path : undefined,
					summary
				}
			})
		}
		dispatch('update', path)
		drawer.closeDrawer()
	}
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Move/Rename {initialPath}" on:close={drawer.closeDrawer}>
		{#if !own}
			<Alert type="warning" title="Not owner">
				Since you do not own this item, you cannot move this item (you can however fork it)
			</Alert>
		{/if}
		<h2 class="border-b pb-1 mt-2 mb-4">Summary</h2>
		<input
			type="text"
			bind:value={summary}
			placeholder="Short summary to be displayed when listed"
			disabled={!own}
		/>

		<h2 class="border-b pb-1 mt-10 mb-4">Path</h2>
		<div class="flex flex-col mb-2 gap-6">
			<Path disabled={!own} {kind} {initialPath} bind:path />
		</div>
		{#snippet actions()}
			<Button disabled={!own} on:click={updatePath}>Move/Rename</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
