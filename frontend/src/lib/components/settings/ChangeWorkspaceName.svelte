<script lang="ts">
	import {
		workspaceStore,
		userWorkspaces,
		usersWorkspaceStore,
		nonMemberWorkspaces
	} from '$lib/stores'
	import { latestRequest } from '$lib/utils/latestRequest'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import Button from '../common/button/Button.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import { untrack } from 'svelte'

	// Forks have no customizable display name — the unique id is the name
	// (same convention as the create-fork form), so the editor is hidden.
	const isFork = $derived(workspaceIsFork($workspaceStore, $userWorkspaces))

	let currentName = $state('')
	let newName = $state('')

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => getWorkspaceName())
		}
	})

	async function getWorkspaceName() {
		currentName = await WorkspaceService.getWorkspaceName({ workspace: $workspaceStore! })
		newName = currentName
	}

	// Overlapping renames: an older list must not land last and restore the previous name.
	const listRefreshes = latestRequest()
	async function renameWorkspace() {
		if (!newName || newName === currentName) {
			return
		}
		const workspace = $workspaceStore!
		const name = newName
		await WorkspaceService.changeWorkspaceName({
			workspace,
			requestBody: {
				new_name: name
			}
		})

		sendUserToast(`Changed workspace name to ${name}`)
		getWorkspaceName()
		// A superadmin's non-member workspace is absent from `listUserWorkspaces`, so the
		// header reads it from this entry instead.
		nonMemberWorkspaces.update(
			(n) =>
				n && {
					...n,
					workspaces: n.workspaces.map((w) => (w.id === workspace ? { ...w, name } : w))
				}
		)
		const isLatest = listRefreshes.start()
		const list = await WorkspaceService.listUserWorkspaces()
		if (isLatest()) usersWorkspaceStore.set(list)
	}
</script>

{#if !isFork}
	<div class="flex flex-col gap-1">
		<p class="font-semibold text-xs text-emphasis">Workspace name</p>
		<p class="text-xs text-secondary font-normal">Displayable name</p>
		<div class="flex flex-row gap-2 items-center">
			<TextInput
				bind:value={newName}
				size="sm"
				class="max-w-xs"
				inputProps={{
					placeholder: 'Workspace name',
					'aria-label': 'Workspace name',
					onkeydown: (e) => {
						if (e.key === 'Enter') {
							renameWorkspace()
						}
					}
				}}
			/>
			<Button
				size="sm"
				variant="accent"
				disabled={!newName || newName === currentName}
				on:click={() => {
					renameWorkspace()
				}}
			>
				Save
			</Button>
		</div>
	</div>
{/if}
