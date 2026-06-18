<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Path from './Path.svelte'
	import { isOwner } from '$lib/utils'
	import { updateItemPathAndSummary, checkFlowOnBehalfOf } from './moveRenameManager'
	import Label from './Label.svelte'
	import TextInput from './text_input/TextInput.svelte'

	const dispatch = createEventDispatcher()

	type Kind = 'script' | 'resource' | 'schedule' | 'variable' | 'flow' | 'app'

	let kind = $state<Kind>('flow')
	let initialPath = $state('')
	let initialSummary = $state('')
	let path = $state<string | undefined>(undefined)
	let summary = $state<string | undefined>(undefined)
	let dirtyPath = $state(false)

	let drawer = $state<Drawer>() as Drawer

	let own = $state(false)
	let onBehalfOfEmail = $state<string | undefined>(undefined)
	let hasChanges = $derived((summary ?? '') !== initialSummary || dirtyPath)

	export async function openDrawer(
		initialPath_l: string,
		summary_l: string | undefined,
		kind_l: Kind
	) {
		kind = kind_l
		path = undefined
		dirtyPath = false
		onBehalfOfEmail = undefined
		initialPath = initialPath_l
		initialSummary = summary_l ?? ''
		summary = summary_l
		loadOwner()
		drawer.openDrawer()
		if (kind === 'flow') {
			onBehalfOfEmail = await checkFlowOnBehalfOf($workspaceStore!, initialPath_l)
		}
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
			<Alert type="warning" title="Not owner" class="mb-4">
				Since you do not own this item, you cannot move this item (you can however fork it)
			</Alert>
		{/if}
		{#if own && onBehalfOfEmail}
			<Alert type="info" title="Run on behalf of" class="mb-4">
				This flow will be redeployed on behalf of you ({$userStore?.email}) instead of {onBehalfOfEmail}
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
			<Path disabled={!own} {kind} {initialPath} bind:path bind:dirty={dirtyPath} />
		</Label>
		{#snippet actions()}
			<Button variant="accent" disabled={!own || !hasChanges} on:click={updatePath}
				>Move/Rename</Button
			>
		{/snippet}
	</DrawerContent>
</Drawer>
