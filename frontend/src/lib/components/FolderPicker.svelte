<script lang="ts">
	import { FolderService } from '$lib/gen'
	import { workspaceStore, userStore } from '$lib/stores'
	import { Eye, PlusIcon } from 'lucide-svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import FolderEditor from './FolderEditor.svelte'
	import Select from './select/Select.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Label from './Label.svelte'
	import { tick } from 'svelte'

	let folders: { name: string; write: boolean }[] = $state([])
	let filterText: string = $state('')
	let selectOpen: boolean = $state(false)
	let nameInput: TextInput | undefined = $state()
	let newFolder: Drawer | null = $state(null)
	let viewFolder: Drawer | null = $state(null)
	let newFolderName: string = $state('')
	let folderCreated: string | undefined = $state(undefined)

	type Props = {
		folderName: string
		initialPath?: string
		disabled?: boolean
		disableEditing?: boolean
		size?: 'sm' | 'md'
		drawerOffset?: number
	}

	let {
		folderName = $bindable(''),
		initialPath = $bindable(undefined),
		disabled = $bindable(undefined),
		disableEditing = $bindable(undefined),
		size = 'md',
		drawerOffset = 0
	}: Props = $props()

	async function loadFolders(): Promise<void> {
		let initialFolders: { name: string; write: boolean }[] = []
		let initialFolder = ''
		if (initialPath?.split('/')?.[0] == 'f') {
			initialFolder = initialPath?.split('/')?.[1]
			initialFolders.push({ name: initialFolder, write: true })
		}

		const excludedFolders = [initialFolder, 'app_groups', 'app_custom', 'app_themes']

		folders = initialFolders.concat(
			(
				await FolderService.listFolderNames({
					workspace: $workspaceStore!
				})
			)
				.filter((x) => !excludedFolders.includes(x))
				.map((x) => ({
					name: x,
					write:
						$userStore?.folders?.includes(x) == true ||
						($userStore?.is_admin ?? false) ||
						($userStore?.is_super_admin ?? false)
				}))
		)
	}

	async function openCreateFolder() {
		newFolderName = filterText
		folderCreated = undefined
		newFolder?.openDrawer()
		await tick()
		nameInput?.focus()
	}

	async function addFolder() {
		await FolderService.createFolder({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newFolderName }
		})
		folderCreated = newFolderName
		$userStore?.folders?.push(newFolderName)
		folderName = newFolderName
		loadFolders()
	}

	let selectItems = $derived(
		folders.map((f) => ({
			value: f.name,
			label: f.name + (f.write ? '' : ' (read-only)'),
			disabled: !f.write
		}))
	)

	let noMatchingItems = $derived(
		filterText &&
			!selectItems.some((item) => item.label.toLowerCase().includes(filterText.toLowerCase()))
	)

	function handleSelectKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && selectOpen && noMatchingItems) {
			e.preventDefault()
			selectOpen = false
			openCreateFolder()
		}
	}

	loadFolders()
</script>

<Drawer bind:this={newFolder} name="newFolder" offset={drawerOffset}>
	<DrawerContent
		title={folderCreated ? `Folder ${folderCreated}` : 'Create folder'}
		on:close={() => {
			newFolder?.closeDrawer()
			folderCreated = undefined
		}}
	>
		{#if folderCreated}
			<FolderEditor name={folderCreated} />
		{:else}
			<div class="flex flex-col gap-4">
				<Label label="Folder name">
					<TextInput
					bind:this={nameInput}
					bind:value={newFolderName}
					inputProps={{
						placeholder: 'folder_name',
						onkeydown: (e: KeyboardEvent) => {
							if (e.key === 'Enter' && newFolderName) {
								e.preventDefault()
								addFolder()
							}
						}
					}}
				/>
				</Label>
				<Button variant="accent" disabled={!newFolderName} on:click={addFolder}>Create</Button>
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={viewFolder} offset={drawerOffset}>
	<DrawerContent title="Folder {folderName}" on:close={viewFolder.closeDrawer}>
		<FolderEditor name={folderName ?? ''} />
	</DrawerContent>
</Drawer>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="flex flex-row items-center gap-1 w-full" onkeydown={handleSelectKeydown}>
	<Select
		bind:value={folderName}
		bind:filterText
		bind:open={selectOpen}
		items={selectItems}
		disabled={disabled || disableEditing}
		{size}
		placeholder="Select folder"
	>
		{#snippet bottomSnippet({ close })}
			<button
				class="sticky py-2 px-4 w-full text-left text-xs font-semibold text-medium hover:bg-surface-hover flex items-center justify-center gap-2 border-t border-border-light {noMatchingItems ? 'bg-surface-hover' : ''}"
				onclick={() => {
					close()
					openCreateFolder()
				}}
			>
				<PlusIcon class="inline" size={16} />
				Create folder
			</button>
		{/snippet}
	</Select>
	<Button
		title="View folder"
		variant="subtle"
		unifiedSize={size}
		disabled={!folderName || folderName == ''}
		on:click={viewFolder.openDrawer}
		iconOnly
		startIcon={{ icon: Eye }}
	/>
</div>
