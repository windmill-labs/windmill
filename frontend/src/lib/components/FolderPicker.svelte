<script lang="ts">
	import { FolderService, UserService, type User } from '$lib/gen'
	import { workspaceStore, userStore } from '$lib/stores'
	import { isDemoWorkspaceRestricted } from '$lib/cloud'
	import { ChevronDown, Pen, PlusIcon } from 'lucide-svelte'
	import { Button } from './common'
	import FolderEditorDrawer from './FolderEditorDrawer.svelte'
	import Select from './select/Select.svelte'
	import { sendUserToast } from '$lib/toast'

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
		/** List and create folders in this workspace instead of the active one. For a
		 * screen that targets a workspace it has not switched to — the project import
		 * wizard picks a destination and only enters it when the import runs. */
		workspace?: string
	}

	let {
		folderName = $bindable(''),
		initialPath = $bindable(undefined),
		disabled = $bindable(undefined),
		disableEditing = $bindable(undefined),
		size = 'md',
		drawerOffset = 0,
		selectInputClass,
		workspace
	}: Props = $props()

	const targetWorkspace = $derived(workspace ?? $workspaceStore ?? '')

	// `$userStore` describes the workspace the app is *in*. When this picker is aimed
	// somewhere else, those memberships answer the wrong question — and since a folder
	// without write access renders disabled, a stale answer makes the real folders
	// unpickable. Resolve the membership for the workspace actually being listed.
	let targetUser: User | undefined = $state(undefined)
	const aimedElsewhere = $derived(!!workspace && workspace !== $workspaceStore)
	const membership = $derived(aimedElsewhere ? targetUser : ($userStore ?? undefined))

	const restricted = $derived(
		isDemoWorkspaceRestricted(targetWorkspace, membership?.is_admin, membership?.is_super_admin)
	)

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
						workspace: targetWorkspace
					})
				)
					.filter((x) => !excludedFolders.includes(x))
					.map((x) => ({
						name: x,
						write:
							membership?.folders?.includes(x) == true ||
							(membership?.is_admin ?? false) ||
							(membership?.is_super_admin ?? false)
					}))
			)
		} catch (e) {
			sendUserToast(`Could not load folders: ${e}`, true)
		} finally {
			loadingFolders = false
		}
	}

	async function onFolderSaved(saved: string, created: boolean) {
		if (created) {
			// The creator owns what they just created. Recorded on whichever membership
			// this picker is reading, and *before* reloading, so the new folder comes
			// back selectable rather than `(read-only)` — `loadFolders` derives `write`
			// from exactly this.
			if (aimedElsewhere) {
				if (targetUser) targetUser.folders = [...(targetUser.folders ?? []), saved]
			} else if ($userStore) {
				// Writing $userStore.folders = [...] would call userStore.set(),
				// which re-triggers Path.svelte's $effect.pre and calls initPath()/reset(),
				// switching the owner toggle from "Folder" back to "User".
				if (!$userStore.folders) $userStore.folders = []
				$userStore.folders.push(saved)
			}
		}
		await loadFolders()
		if (created) folderName = saved
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

	async function loadTargetUser(): Promise<void> {
		if (!workspace || workspace === $workspaceStore) return
		try {
			targetUser = await UserService.whoami({ workspace })
		} catch {
			// Not a member, or the call failed: every folder stays read-only, which is
			// the safe reading — the import would be refused anyway.
			targetUser = undefined
		}
	}

	loadTargetUser().then(loadFolders)
</script>

<FolderEditorDrawer
	bind:this={folderEditorDrawer}
	offset={drawerOffset}
	workspace={targetWorkspace}
	onSaved={onFolderSaved}
/>

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
