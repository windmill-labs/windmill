<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Path from './Path.svelte'
	import { isOwner } from '$lib/utils'
	import { updateItemPathAndSummary } from './moveRenameManager'
	import Label from './Label.svelte'
	import TextInput from './text_input/TextInput.svelte'

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
		if (kind === 'flow' || kind === 'script' || kind === 'app') {
			await updateItemPathAndSummary({
				workspace: $workspaceStore!,
				kind,
				initialPath,
				newPath: path ?? '',
				newSummary: summary ?? ''
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
		<Label label="Summary" class="mb-6">
			<TextInput
				inputProps={{
					type: 'text',
					placeholder: 'Short summary to be displayed when listed',
					disabled: !own
				}}
				bind:value={summary}
			/>
		</Label>

		<Label label="Path">
			<Path disabled={!own} {kind} {initialPath} bind:path />
		</Label>
		{#snippet actions()}
			<Button variant="accent" disabled={!own} on:click={updatePath}>Move/Rename</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
