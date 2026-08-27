<script lang="ts">
	import { FolderService } from '$lib/gen'
	import { workspaceStore, userStore } from '$lib/stores'
	import { isDemoWorkspaceRestricted } from '$lib/cloud'
	import { ChevronDown, Pen, PlusIcon } from 'lucide-svelte'
	import { Button } from './common'
	import FolderEditorDrawer from './FolderEditorDrawer.svelte'
	import Select from './select/Select.svelte'
	import { sendUserToast } from '$lib/toast'

	const restricted = $derived(
		isDemoWorkspaceRestricted($workspaceStore, $userStore?.is_admin, $userStore?.is_super_admin)
	)

	let folders: { name: string; write: boolean }[] = $state([])
	let filterText: string = $state('')
	let selectOpen: boolean = $state(false)
	let folderEditorDrawer: FolderEditorDrawer | undefined = $state()
	let loadingFolders: boolean = $state(true)

	type Props = {
		folderName: string
		initialPath?: string
		disabled?: boolean
		disableEditing?: boolean
		size?: 'sm' | 'md'
		drawerOffset?: number
		selectInputClass?: string
	}

	let {
		folderName = $bindable(''),
		initialPath = $bindable(undefined),
		disabled = $bindable(undefined),
		disableEditing = $bindable(undefined),
		size = 'md',
		drawerOffset = 0,
		selectInputClass
	}: Props = $props()

	async function loadFolders(): Promise<void> {
		loadingFolders = true
		try {
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
		} catch (e) {
			sendUserToast(`Could not load folders: ${e}`, true)
		} finally {
			loadingFolders = false
		}
	}

	async function onFolderSaved(saved: string, created: boolean) {
		await loadFolders()
		if (!created) return
		folderName = saved

		// Writing $userStore.folders = [...] would call userStore.set(),
		// which re-triggers Path.svelte's $effect.pre and calls initPath()/reset(),
		// switching the owner toggle from "Folder" back to "User".
		if ($userStore) {
			if (!$userStore.folders) $userStore.folders = []
			$userStore.folders.push(saved)
		}
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
		if (e.key === 'Enter' && selectOpen && noMatchingItems && !restricted) {
			e.preventDefault()
			selectOpen = false
			folderEditorDrawer?.initNew(filterText)
		}
	}

	loadFolders()
</script>

<FolderEditorDrawer bind:this={folderEditorDrawer} offset={drawerOffset} onSaved={onFolderSaved} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="flex group flex-row w-full items-center relative"
	role="group"
	onkeydown={handleSelectKeydown}
>
	<Select
		useContentEditable
		bind:value={folderName}
		bind:filterText
		bind:open={selectOpen}
		items={selectItems}
		disabled={disabled || disableEditing}
		loading={loadingFolders}
		{size}
		placeholder="Select folder"
		class="grow min-w-0"
		inputClass={selectInputClass}
		RightIcon={ChevronDown}
	>
		{#snippet endSnippet({ item, close })}
			<Button
				disabled={disabled || disableEditing}
				variant="subtle"
				unifiedSize="xs"
				wrapperClasses="-mr-2 pl-1 -my-2"
				btnClasses="hover:bg-surface-tertiary"
				onClick={() => {
					folderEditorDrawer?.initEdit(item.value ?? '')
					close()
				}}
				startIcon={{ icon: Pen }}
				iconOnly
			/>
		{/snippet}
		{#snippet bottomSnippet({ close })}
			{#if !restricted}
				<button
					class="sticky py-2 px-4 w-full text-left text-xs font-medium hover:bg-surface-hover flex items-center justify-center gap-2 border-t border-border-light {noMatchingItems
						? 'bg-surface-hover'
						: ''}"
					onclick={() => {
						close()
						folderEditorDrawer?.initNew(filterText)
					}}
				>
					<PlusIcon class="inline" size={16} />
					Create folder
				</button>
			{/if}
		{/snippet}
	</Select>
</div>
