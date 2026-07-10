<script lang="ts">
	import { workspaceStore, userWorkspaces } from '$lib/stores'
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

	async function renameWorkspace() {
		if (!newName || newName === currentName) {
			return
		}
		await WorkspaceService.changeWorkspaceName({
			workspace: $workspaceStore!,
			requestBody: {
				new_name: newName
			}
		})

		sendUserToast(`Changed workspace name to ${newName}`)
		getWorkspaceName()
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
