<script lang="ts">
	import { FolderService } from '$lib/gen'
	import { workspaceStore, userStore } from '$lib/stores'
	import { Plus, Eye } from 'lucide-svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import FolderEditor from './FolderEditor.svelte'

	let initialPath: string | undefined = $state(undefined)
	let folders: { name: string; write: boolean }[] = $state([])
	let newFolder: Drawer | null = $state(null)
	let viewFolder: Drawer | null = $state(null)
	let newFolderName: string = $state('')
	let folderCreated: string | undefined = $state(undefined)

	let { folderName = $bindable('') } = $props()

	async function loadFolders(): Promise<void> {
		let initialFolders: { name: string; write: boolean }[] = []
		let initialFolder = ''
		if (initialPath?.split('/')?.[0] == 'f') {
			initialFolder = initialPath?.split('/')?.[1]
			initialFolders.push({ name: initialFolder, write: true })
		}
		folders = initialFolders.concat(
			(
				await FolderService.listFolderNames({
					workspace: $workspaceStore!
				})
			)
				.filter(
					(x) =>
						x != initialFolder &&
						x != 'app_groups' &&
						x != 'app_custom' &&
						x != 'app_themes' &&
						x != 'app_custom'
				)
				.map((x) => ({
					name: x,
					write:
						$userStore?.folders?.includes(x) == true ||
						($userStore?.is_admin ?? false) ||
						($userStore?.is_super_admin ?? false)
				}))
		)
	}

	async function addFolder() {
		await FolderService.createFolder({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newFolderName }
		})
		folderCreated = newFolderName
		$userStore?.folders?.push(newFolderName)
		loadFolders()
	}

	loadFolders()
</script>

<Drawer bind:this={newFolder}>
	<DrawerContent
		title="New Folder"
		on:close={() => {
			newFolder?.closeDrawer()
			folderCreated = undefined
		}}
	>
		{#if !folderCreated}
			<div class="flex flex-col gap-2">
				<input placeholder="New folder name" bind:value={newFolderName} />
				<Button size="md" startIcon={{ icon: Plus }} disabled={!newFolderName} on:click={addFolder}>
					New&nbsp;folder
				</Button>
			</div>
		{:else}
			<FolderEditor name={folderCreated} />
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={viewFolder}>
	<DrawerContent title="Folder {folderName}" on:close={viewFolder.closeDrawer}>
		<FolderEditor name={folderName ?? ''} />
	</DrawerContent>
</Drawer>

<div class="flex flex-row items-center gap-1 w-full">
	<select class="grow w-full" bind:value={folderName}>
		{#if folders?.length == 0}
			<option disabled>No folders</option>
		{/if}
		{#each folders as { name, write }}
			<option disabled={!write}>{name}{write ? '' : ' (read-only)'}</option>
		{/each}
	</select>
	<Button
		title="View folder"
		btnClasses="!p-1.5"
		variant="border"
		color="light"
		size="xs"
		disabled={!folderName || folderName == ''}
		on:click={viewFolder.openDrawer}
		iconOnly
		startIcon={{ icon: Eye }}
	/>
	<Button
		title="New folder"
		btnClasses="!p-1.5"
		variant="border"
		color="light"
		size="xs"
		on:click={newFolder.openDrawer}
		iconOnly
		startIcon={{ icon: Plus }}
	/>
</div>
