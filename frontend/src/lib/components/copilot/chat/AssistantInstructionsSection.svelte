<!--
@component
The Instructions section of the assistant settings modal: every block of instructions the
assistant follows, in one list — the workspace one an admin sets for everyone, the user one
stored in this browser, then each folder's `ai_instruction` resources in a tree. A row opens
its block on a detail page laid over the list, like a skill in the Skills section.
-->
<script lang="ts">
	import { Button, ListRow, Section } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import PagedContent from '$lib/components/common/modal/PagedContent.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { markdownProse } from '$lib/components/markdownProse'
	import {
		copilotInfo,
		getUserCustomPrompts,
		setCopilotInfo,
		setUserCustomPrompts
	} from '$lib/aiStore'
	import { ResourceService, WorkspaceService } from '$lib/gen'
	import { userStore, type UserExt } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import {
		ArrowLeft,
		Building2,
		ChevronDown,
		ChevronRight,
		ExternalLink,
		Eye,
		Folder,
		FolderTree,
		Pencil,
		Plus,
		RotateCcw,
		ScrollText,
		Trash2,
		User
	} from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import {
		listFolderInstructionRows,
		readFolderInstructionBody,
		saveFolderInstruction,
		type FolderInstructionRow
	} from './folderInstructions'
	import { buildSkillTree, countSkills, type SkillTreeNode } from './skills/skillTree'

	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)

	let {
		ws,
		active,
		blocksClose = $bindable()
	}: {
		/** The workspace the chat operates on, which is not always the one on screen. */
		ws: string
		/** Whether this is the panel on screen. The list costs a settings read and a
		 * resource listing, so it is fetched when someone looks at instructions rather
		 * than on every open of the modal. */
		active: boolean
		/** True while this section is in the middle of something the modal must not
		 * close under: a detail page, instructions typed and not yet saved, a dialog. */
		blocksClose: boolean
	} = $props()

	const MAX_PROMPT_LENGTH = 5000

	const aiChatManager = getAiChatManager()
	// The section acts on `ws`, which in a session is not the nav workspace, so the link
	// has to name it or it opens settings the user may not administer.
	let aiSettingsHref = $derived(`/workspace_settings?workspace=${ws}&tab=ai`)

	let mode = $derived(aiChatManager.mode)
	// `actingUser?.is_admin` is the role in the nav workspace, not necessarily in `ws`, so
	// the resolved role is keyed to the workspace it was read for, and an unresolved one
	// reads as no admin: offering the field and taking it away on resolve would discard
	// whatever was typed in between. Superadmin holds everywhere.
	let targetRole = $state<{ workspace: string; user: UserExt | undefined } | undefined>(undefined)
	let roleRead = $derived(targetRole?.workspace === ws ? targetRole : undefined)
	let navIsTarget = $derived($userStore?.workspace_id === ws)
	let roleForTarget = $derived(roleRead?.user)
	let roleResolved = $derived(roleRead !== undefined || navIsTarget)
	// A read that came back with no user: read-only like a non-admin, but said differently,
	// since a failed lookup is not evidence of the role it failed to read.
	let roleUnknown = $derived(roleRead !== undefined && !roleRead.user && !navIsTarget)
	let isAdmin = $derived(
		Boolean(
			actingUser?.is_super_admin ||
				(roleForTarget ? roleForTarget.is_admin : navIsTarget && actingUser?.is_admin)
		)
	)

	// A session whose fork is still staged has no workspace of its own yet, so `ws`
	// resolves to the PARENT. Saving here would edit the live parent's shared AI config,
	// which every chat still in it would then be given.
	function pendingForkParent(): string | undefined {
		return aiChatManager.sessionContextResolver?.()?.pendingForkOf
	}
	let forkPending = $derived(pendingForkParent() !== undefined)

	// What is on screen, against what was loaded. Two pairs rather than one: the halves
	// are stored in different places and save through different calls, and only the one
	// that changed should be written.
	let workspaceDraft = $state('')
	let workspaceSaved = $state('')
	let userDraft = $state('')
	let userSaved = $state('')
	let loading = $state(false)
	let saving = $state(false)

	// True when the workspace has no AI providers of its own (it uses instance defaults).
	// In that case the backend never makes workspace custom_prompts effective, so a saved
	// workspace prompt would be dead config — mirror the settings page and show it read-only.
	let workspaceMissingProviders = $state(false)
	let workspaceReadOnly = $derived(!isAdmin || workspaceMissingProviders || forkPending)
	let readOnlyReason = $derived(
		forkPending
			? `This session has not created its workspace yet, so these would be saved to "${pendingForkParent()}" and given to every chat already in it. Send a message first.`
			: !roleResolved
				? `Checking your access to ${ws}.`
				: roleUnknown
					? `Could not read your access to ${ws}. Reopen this section to try again.`
					: !isAdmin
						? 'Only workspace admins can edit the workspace instructions.'
						: 'This workspace uses instance AI defaults, so a workspace prompt would have no effect. Configure workspace AI providers in AI settings first.'
	)

	let workspaceChanged = $derived(!workspaceReadOnly && workspaceDraft !== workspaceSaved)
	let userChanged = $derived(userDraft !== userSaved)

	type Detail =
		| { kind: 'workspace' }
		| { kind: 'user' }
		| { kind: 'folder'; row: FolderInstructionRow }
		| { kind: 'new' }
	/** The block the detail page holds, with the workspace it was opened in: a folder's
	 * instructions read in A must not stay on screen, and be saved, after a switch to B.
	 * Kept after the page closes, so the page sliding away still shows what it held. */
	let opened = $state<{ workspace: string; detail: Detail } | undefined>(undefined)
	let detailOpen = $state(false)
	let detail = $derived(opened?.workspace === ws ? opened.detail : undefined)
	let showingDetail = $derived(detailOpen && detail !== undefined)

	let folderRows = $state<FolderInstructionRow[]>([])
	let folderLoadError = $state<string | undefined>(undefined)
	let tree = $derived(buildSkillTree(folderRows))
	/** Folders the user closed, keyed by full prefix. */
	let collapsed = $state<Record<string, boolean>>({})

	// The folder instruction on the detail page: what Monaco holds against what was read.
	let folderContent = $state('')
	let folderOriginal = $state('')
	let folderMode = $state<'view' | 'edit'>('view')
	let newPath = $state('')
	let newPathError = $state('')
	let editorSeq = $state(0)
	let bodyEditor: SimpleEditor | undefined = $state(undefined)
	/** Bumped on every open of a folder instruction or a new one, so a read or a save that
	 * lands after the user moved to another one leaves that one's editor alone. */
	let visit = 0
	let toDelete = $state<FolderInstructionRow | undefined>(undefined)
	let folderChanged = $derived(
		detail?.kind === 'new'
			? folderContent.trim() !== ''
			: detail?.kind === 'folder' && folderContent !== folderOriginal
	)

	let dirty = $derived(workspaceChanged || userChanged || folderChanged)
	/** Whether the block on the detail page has unsaved text. */
	let pageChanged = $derived(
		detail?.kind === 'workspace'
			? workspaceChanged
			: detail?.kind === 'user'
				? userChanged
				: folderChanged
	)

	$effect(() => {
		blocksClose = dirty || showingDetail || toDelete !== undefined
	})

	// Loaded on the first look and again on a workspace switch: B's instructions must not
	// be saved over A's. A reload drops a draft, so it waits until one is saved or
	// reverted — hence `dirty` tracked, and `loadedWorkspace` to pick the wait up against
	// whatever `ws` is by then (a staged fork commits into a workspace of its own).
	let loadSeq = 0
	let loadedWorkspace: string | undefined = undefined
	$effect(() => {
		const target = ws
		const shown = active
		const clean = !dirty
		untrack(() => {
			// Parked: read again on the next visit, since every block is editable from
			// elsewhere too.
			if (!shown) {
				loadedWorkspace = undefined
				return
			}
			if (!target || !clean || target === loadedWorkspace) return
			if (loadedWorkspace !== undefined) collapsed = {}
			loadedWorkspace = target
			void load(target)
		})
	})

	async function load(target: string) {
		const seq = ++loadSeq
		loading = true
		try {
			const resolved = await getUserExt(target).catch(() => undefined)
			if (seq !== loadSeq) return
			targetRole = { workspace: target, user: resolved }
			const user = getUserCustomPrompts()[mode] ?? ''
			// Seeded from the same source `saveWorkspace` writes to (the raw workspace
			// ai_config), which also says whether the workspace has providers of its own.
			// Non-admins cannot read raw settings, so they get the effective prompt.
			let workspace = $copilotInfo.customPrompts?.[mode] ?? ''
			let missingProviders = false
			if (isAdmin) {
				try {
					const settings = await WorkspaceService.getSettings({ workspace: target })
					missingProviders = Object.keys(settings?.ai_config?.providers ?? {}).length === 0
					workspace = settings?.ai_config?.custom_prompts?.[mode] ?? ''
				} catch (err) {
					sendUserToast(`Failed to load workspace AI prompt: ${err}`, true)
				}
			}
			await loadFolders(target, resolved)
			if (seq !== loadSeq) return
			workspaceMissingProviders = missingProviders
			// Read before the assignments below move the baseline they compare against.
			// Each half is then seeded only while it still holds what it was loaded with:
			// the effect that starts this checks `dirty` before the request, not after it,
			// so an admin who opens the block and types straight away would otherwise have
			// the settings response land on top of the text they are in the middle of.
			const keepWorkspaceDraft = workspaceChanged
			const keepUserDraft = userChanged
			workspaceSaved = workspace
			if (!keepWorkspaceDraft) workspaceDraft = workspace
			userSaved = user
			if (!keepUserDraft) userDraft = user
		} finally {
			if (seq === loadSeq) loading = false
		}
	}

	let folderSeq = 0
	async function loadFolders(target: string, user = roleRead?.user) {
		const seq = ++folderSeq
		try {
			// The role read for `target`, not the nav workspace's: it decides which rows
			// offer Edit.
			const rows = await listFolderInstructionRows(target, user ?? $userStore ?? undefined)
			if (seq !== folderSeq || target !== ws) return
			folderRows = rows
			folderLoadError = undefined
		} catch (e) {
			if (seq !== folderSeq || target !== ws) return
			folderRows = []
			folderLoadError = e.body ?? e.message
		}
	}

	/** Escape backs out one step rather than closing the modal: `blocksClose` holds the
	 * modal shut meanwhile. On a detail page it first puts unsaved text back, then
	 * returns to the list; on the list it puts back drafts left there. */
	function onKeydown(event: KeyboardEvent) {
		// Only while this is the section on screen: every section stays mounted, so an
		// Escape meant for another one would revert these drafts with nothing visible
		// to say that it had.
		if (!active || event.key !== 'Escape' || toDelete !== undefined) return
		if (showingDetail) {
			event.preventDefault()
			event.stopPropagation()
			if (pageChanged) revertPage()
			else closeDetail()
			return
		}
		if (!dirty) return
		event.preventDefault()
		event.stopPropagation()
		workspaceDraft = workspaceSaved
		userDraft = userSaved
		setFolderContent(folderOriginal)
	}

	function revertPage() {
		if (detail?.kind === 'workspace') workspaceDraft = workspaceSaved
		else if (detail?.kind === 'user') userDraft = userSaved
		else setFolderContent(folderOriginal)
	}

	function openBlock(kind: 'workspace' | 'user') {
		opened = { workspace: ws, detail: { kind } }
		detailOpen = true
	}

	/** Both the state the form reads and what Monaco shows: `SimpleEditor` takes `code`
	 * when it builds its model and never again, and the detail page stays mounted. */
	function setFolderContent(next: string) {
		folderContent = next
		bodyEditor?.setCode(next)
	}

	/** Lands on the rendered instructions: reading them is the common reason to open
	 * them, and the header's View/Edit switch is one click from the other half. */
	async function openFolder(row: FolderInstructionRow) {
		// Back to the row whose draft the list marks unsaved: the draft, not the server body.
		if (detail?.kind === 'folder' && detail.row.path === row.path && folderChanged) {
			detailOpen = true
			return
		}
		// Pinned across the await, and re-checked after it: a switch in between must not
		// open A's body under B, where Save would write it.
		const source = ws
		const seq = ++visit
		try {
			const body = await readFolderInstructionBody(source, row.path)
			if (source !== ws || seq !== visit) return
			setFolderContent(body)
			folderOriginal = body
			folderMode = 'view'
			opened = { workspace: source, detail: { kind: 'folder', row } }
			detailOpen = true
		} catch (e) {
			sendUserToast(`Failed to load ${row.path}: ${e.body ?? e.message}`, true)
		}
	}

	function openNew() {
		if (blockedByPendingFork()) return
		visit++
		editorSeq++
		newPath = ''
		setFolderContent('')
		folderOriginal = ''
		folderMode = 'edit'
		opened = { workspace: ws, detail: { kind: 'new' } }
		detailOpen = true
	}

	function closeDetail() {
		detailOpen = false
	}

	/** Left and Right step between the two pages, which is `PagedContent` answering the
	 * arrows once it is given this. Forward only goes somewhere once a block was opened. */
	function navigate(key: string) {
		if (key === 'list') closeDetail()
		else if (detail !== undefined) detailOpen = true
	}

	function blockedByPendingFork(): boolean {
		const parent = pendingForkParent()
		if (parent === undefined) return false
		sendUserToast(
			`This session has not created its workspace yet, so the instructions would be written to "${parent}" instead. Send a message first.`,
			true
		)
		return true
	}

	/** The chat reads the folder list on its next send; told now, so its system prompt
	 * names a folder just given instructions. Never mid-turn: the refresh would rewrite
	 * the system prompt underneath a running turn. */
	function refreshChat(target: string) {
		if (!aiChatManager.loading && !aiChatManager.sendInFlight) {
			void aiChatManager.refreshGlobalFolderInstructions(target)
		}
	}

	async function saveFolder() {
		const current = detail
		if (current?.kind !== 'folder' && current?.kind !== 'new') return
		if (blockedByPendingFork()) return
		const target = ws
		const path = current.kind === 'new' ? newPath : current.row.path
		// Monaco hands its text to `folderContent` on a debounce, so the last keystrokes
		// before a quick click would otherwise be left out of the save.
		bodyEditor?.flushPendingChanges()
		const content = folderContent
		const seq = visit
		saving = true
		try {
			await saveFolderInstruction(target, path, content, { create: current.kind === 'new' })
			sendUserToast(current.kind === 'new' ? `Added ${path}` : `Saved ${path}`)
			await loadFolders(target)
			refreshChat(target)
			// Another instruction opened meanwhile owns the editor now.
			if (seq !== visit) return
			folderOriginal = content
			if (current.kind === 'new') {
				const row = folderRows.find((r) => r.path === path)
				if (row) opened = { workspace: target, detail: { kind: 'folder', row } }
				folderMode = 'view'
			}
		} catch (e) {
			// The editor keeps what was typed, so the save can be retried.
			sendUserToast(`Failed to save ${path}: ${e.body ?? e.message}`, true)
		} finally {
			saving = false
		}
	}

	async function removeFolder(row: FolderInstructionRow) {
		if (blockedByPendingFork()) return
		const target = ws
		try {
			await ResourceService.deleteResource({ workspace: target, path: row.path })
			sendUserToast(`Deleted ${row.path}`)
			setFolderContent('')
			folderOriginal = ''
			closeDetail()
			await loadFolders(target)
			refreshChat(target)
		} catch (e) {
			sendUserToast(`Failed to delete ${row.path}: ${e.body ?? e.message}`, true)
		}
	}

	async function saveBlock() {
		saving = true
		try {
			if (detail?.kind === 'user' && userChanged) saveUser(userDraft.trim())
			if (detail?.kind === 'workspace' && workspaceChanged)
				await saveWorkspace(workspaceDraft.trim())
		} finally {
			saving = false
		}
	}

	function saveUser(value: string) {
		const prompts = getUserCustomPrompts()
		if (value) {
			prompts[mode] = value
		} else {
			delete prompts[mode]
		}
		setUserCustomPrompts(prompts)
		// These live in localStorage, which nothing observes, so the chat is told rather
		// than left to pick them up on its next send. `update_user_instructions` already
		// rebuilds this way when the assistant edits the same block.
		aiChatManager.rebuildGlobalSystemMessage()
		userSaved = value
		userDraft = value
		sendUserToast('User instructions were saved')
	}

	async function saveWorkspace(value: string) {
		const parent = pendingForkParent()
		if (parent !== undefined) {
			sendUserToast(
				`This session has not created its workspace yet, so the instructions would be saved to "${parent}". Send a message first.`,
				true
			)
			return
		}
		// Pinned across both awaits: the read and the write have to land on one workspace.
		const target = ws
		try {
			// Saving prompts requires a full ai_config round-trip; fetch the current config
			// so we don't clobber providers/models/etc.
			const settings = await WorkspaceService.getSettings({ workspace: target })
			const config = settings.ai_config ?? {}
			const custom_prompts = { ...(config.custom_prompts ?? {}) }
			if (value) {
				custom_prompts[mode] = value
			} else {
				delete custom_prompts[mode]
			}
			const response = await WorkspaceService.editCopilotConfig({
				workspace: target,
				requestBody: { ...config, custom_prompts }
			})
			setCopilotInfo(response.effective_ai_config)
			workspaceSaved = value
			workspaceDraft = value
			sendUserToast('Workspace instructions were saved')
		} catch (err) {
			// The field keeps what was typed, so the save can be retried.
			sendUserToast(`Failed to save workspace AI prompt: ${err}`, true)
		}
	}

	/** The first line of a block, for its row. */
	function preview(text: string): string | undefined {
		return text.trim().split('\n')[0] || undefined
	}
</script>

<svelte:window onkeydown={onKeydown} />

<!-- The list and the detail page are levels of one panel, so moving between them slides
     rather than cuts. Warmed once this panel is on screen: the detail page carries a Monaco
     instance, and building it on the first click lands that work inside the transition. -->
<PagedContent
	warm={active}
	class="grow min-h-0"
	current={showingDetail ? 'detail' : 'list'}
	onNavigate={active ? navigate : undefined}
	pages={[
		{ key: 'list', content: listPage },
		{ key: 'detail', content: detailPage }
	]}
/>

{#snippet listPage()}
	<!-- Stable gutter: folding a folder can take the list under the scrollable height, and
	     every row would otherwise jump sideways as the scrollbar came and went. -->
	<div class="grow min-h-0 overflow-y-auto pr-2" style="scrollbar-gutter: stable;">
		<Section
			label="Instructions"
			description="What the assistant follows on top of its own prompt. The workspace's and yours are in every system prompt; a folder's are in it while the session works in that folder, and otherwise reach it the first time it works on an item there."
		>
			{#snippet action()}
				<div class="flex items-center gap-2 shrink-0">
					{#if isAdmin}
						<Button
							href={aiSettingsHref}
							target="_blank"
							variant="subtle"
							unifiedSize="sm"
							endIcon={{ icon: ExternalLink }}
						>
							AI settings
						</Button>
					{/if}
					<Button
						variant="accent"
						unifiedSize="sm"
						startIcon={{ icon: Plus }}
						disabled={saving || forkPending}
						onClick={openNew}
					>
						New folder instructions
					</Button>
				</div>
			{/snippet}

			<div class="flex flex-col gap-0.5">
				{@render block('workspace')}
				{@render block('user')}
				{#each tree as node (node.path)}
					{@render folder(node, true)}
				{/each}
			</div>
			{#if folderLoadError}
				<div class="text-xs text-red-600 dark:text-red-400 px-2 pt-2">
					Failed to load folder instructions: {folderLoadError}
				</div>
			{:else if !loading && folderRows.length === 0}
				<p class="text-2xs text-hint px-2 pt-2">
					No folder has instructions yet. A folder's instructions are followed for every item in it.
				</p>
			{/if}
		</Section>
	</div>
{/snippet}

{#snippet block(kind: 'workspace' | 'user')}
	{@const saved = kind === 'workspace' ? workspaceSaved : userSaved}
	{@const changed = kind === 'workspace' ? workspaceChanged : userChanged}
	{#snippet icon()}
		{#if kind === 'workspace'}
			<Building2 size={16} class="text-tertiary" />
		{:else}
			<User size={16} class="text-tertiary" />
		{/if}
	{/snippet}
	{#snippet title()}
		<span class="truncate leading-5">{kind === 'workspace' ? 'Workspace' : 'You'}</span>
	{/snippet}
	{#snippet subtitle()}
		{#if changed}
			<span class="text-hint">Unsaved changes ·</span>
		{/if}
		{preview(saved) ?? (kind === 'workspace' ? `None set for ${ws}` : 'None set')}
	{/snippet}
	<ListRow {icon} {title} {subtitle} onClick={() => openBlock(kind)} />
{/snippet}

<!-- One folder and everything under it. Recursive: a path may name folders below the `u/x`
     or `f/x` root it starts from. -->
{#snippet folder(node: SkillTreeNode<FolderInstructionRow>, root: boolean)}
	<div>
		<button
			type="button"
			class="w-full flex items-center gap-2 px-2 py-2 text-left rounded-md hover:bg-surface-hover"
			aria-expanded={!collapsed[node.path]}
			onclick={() => (collapsed[node.path] = !collapsed[node.path])}
		>
			{#if collapsed[node.path]}
				<ChevronRight size={14} class="text-tertiary shrink-0" />
			{:else}
				<ChevronDown size={14} class="text-tertiary shrink-0" />
			{/if}
			{#if root && node.path.startsWith('u/')}
				<User size={14} class="text-secondary shrink-0" />
			{:else if root}
				<Folder size={14} class="text-secondary shrink-0" />
			{:else}
				<FolderTree size={14} class="text-secondary shrink-0" />
			{/if}
			<!-- Explicitly normal: buttons carry a heavier weight the label would inherit. -->
			<span class="text-xs font-normal text-emphasis truncate">{node.label}</span>
			<span class="text-2xs text-secondary">{countSkills(node)}</span>
		</button>
		{#if !collapsed[node.path]}
			<div class="pl-4 flex flex-col gap-0.5">
				{#each node.children as child (child.path)}
					{@render folder(child, false)}
				{/each}
				{#each node.skills as row (row.path)}
					{@render instruction(row)}
				{/each}
			</div>
		{/if}
	</div>
{/snippet}

{#snippet instruction(row: FolderInstructionRow)}
	{#snippet icon()}
		<ScrollText size={16} class="text-tertiary" />
	{/snippet}
	{#snippet title()}
		<span class="truncate leading-5">{row.name}</span>
	{/snippet}
	{#snippet subtitle()}
		{#if folderChanged && detail?.kind === 'folder' && detail.row.path === row.path}
			<span class="text-hint">Unsaved changes ·</span>
		{/if}
		{@const inScope = aiChatManager.workingScope?.roots?.find((r) => `${r}/`.startsWith(row.scope))}
		{#if inScope}
			<span class="text-accent">In the prompt while working in {inScope} ·</span>
		{/if}
		Followed for every item under {row.scope}
	{/snippet}
	<ListRow {icon} {title} {subtitle} onClick={() => openFolder(row)} />
{/snippet}

{#snippet field(p: { value: string; readOnly: boolean; onInput: (v: string) => void })}
	<div class="flex flex-col gap-1">
		<TextInput
			value={p.value}
			underlyingInputEl="textarea"
			size="sm"
			class="min-h-24 resize-y"
			inputProps={{
				placeholder: p.readOnly ? '' : 'Anything the assistant should always keep in mind',
				rows: 4,
				maxlength: MAX_PROMPT_LENGTH,
				// Also while saving: the write spans two requests and its success path puts the
				// submitted value back, so text typed in between would be swallowed. `load`
				// keeps a newer draft instead, because it starts on its own and must not block
				// someone who opened the block to type.
				readonly: p.readOnly || saving,
				oninput: (e) => p.onInput(e.currentTarget.value)
			}}
		/>
		{#if !p.readOnly}
			<span class="self-end text-2xs text-hint">
				{p.value.length}/{MAX_PROMPT_LENGTH} characters
			</span>
		{/if}
	</div>
{/snippet}

{#snippet detailPage()}
	<!-- Same reserved gutter as the list, so the slide between the pages does not shift what
	     is under the cursor. -->
	<div class="grow min-h-0 overflow-y-auto pr-2" style="scrollbar-gutter: stable;">
		<!-- Sticky so the way back is always one click away, however far the page scrolls. -->
		<div class="flex sticky top-0 z-10 bg-surface pb-1">
			<Button
				variant="subtle"
				unifiedSize="xs"
				startIcon={{ icon: ArrowLeft }}
				btnClasses="text-secondary"
				onClick={closeDetail}
			>
				Instructions
			</Button>
		</div>
		{#if detail?.kind === 'workspace' || detail?.kind === 'user'}
			{@const kind = detail.kind}
			<Section
				label={kind === 'workspace' ? 'Workspace instructions' : 'Your instructions'}
				wrapperClass="mt-1"
				class="flex flex-col gap-2"
			>
				{#snippet action()}
					<Button
						variant="accent"
						unifiedSize="sm"
						disabled={!pageChanged || saving || loading}
						onClick={saveBlock}
					>
						Save
					</Button>
				{/snippet}
				<Description>
					{kind === 'workspace'
						? `In every system prompt in ${ws}, for everyone.`
						: 'Stored in this browser and sent in every workspace, so they follow you rather than the workspace.'}
				</Description>
				{#if kind === 'workspace'}
					{#if workspaceReadOnly}
						<Alert type="info" title="These are read-only for you" size="xs">
							{readOnlyReason}
						</Alert>
					{/if}
					{@render field({
						value: workspaceDraft,
						readOnly: workspaceReadOnly,
						onInput: (v) => (workspaceDraft = v)
					})}
				{:else}
					{@render field({ value: userDraft, readOnly: false, onInput: (v) => (userDraft = v) })}
				{/if}
			</Section>
		{/if}
		<!-- Mounted whatever the page holds and only hidden: an `{#if}` would dispose Monaco on
		     every visit to the workspace or user block and rebuild it on the way back. -->
		<div class={detail?.kind === 'folder' || detail?.kind === 'new' ? 'block' : 'hidden'}>
			<Section
				label={detail?.kind === 'folder' ? detail.row.path : 'New folder instructions'}
				wrapperClass="mt-1"
				headerClass="min-w-0 truncate pr-2"
			>
				{#snippet action()}
					{#if detail?.kind === 'folder'}
						<div class="flex justify-end shrink-0">
							<ToggleButtonGroup bind:selected={folderMode}>
								{#snippet children({ item })}
									<ToggleButton value="view" label="View" icon={Eye} {item} small />
									<ToggleButton
										value="edit"
										label="Edit"
										icon={Pencil}
										{item}
										small
										disabled={!detail?.row.canWrite || forkPending}
									/>
								{/snippet}
							</ToggleButtonGroup>
						</div>
					{/if}
				{/snippet}
				<div class="w-full flex flex-col gap-3">
					{#if detail?.kind === 'folder'}
						<Description>
							Followed for every item under {detail.row.scope}, delivered the first time the
							assistant works on one of them.
						</Description>
					{/if}
					<div class="{folderMode === 'view' ? 'block' : 'hidden'} {markdownProse.sm}">
						<div class="border rounded-md p-3 overflow-auto space-y-2">
							{#if folderContent.trim()}
								<Markdown md={folderContent} plugins={[gfmPlugin()]} />
							{:else}
								<span class="text-xs text-hint">Empty.</span>
							{/if}
						</div>
					</div>
					<div class="{folderMode === 'edit' ? 'flex' : 'hidden'} flex-col gap-3">
						{#if detail?.kind === 'new'}
							<Label label="Path">
								<span class="text-xs font-normal text-secondary">
									The folder decides what these cover: every item under it, for everyone who can
									read them.
								</span>
								{#key editorSeq}
									<Path
										bind:path={newPath}
										bind:error={newPathError}
										initialPath=""
										fullNamePlaceholder="AGENTS"
										kind="resource"
										workspaceOverride={ws}
										autofocus={false}
									/>
								{/key}
							</Label>
						{/if}
						<div class="border border-border-light rounded-md overflow-hidden">
							<SimpleEditor
								bind:this={bodyEditor}
								autoHeight
								lang="markdown"
								bind:code={folderContent}
								fixedOverflowWidgets={false}
								class="min-h-24"
							/>
						</div>
						<div class="flex items-center justify-between gap-2">
							<div>
								{#if detail?.kind === 'folder'}
									{@const row = detail.row}
									<Button
										variant="subtle"
										unifiedSize="sm"
										startIcon={{ icon: Trash2 }}
										disabled={saving || !row.canWrite || forkPending}
										onClick={() => (toDelete = row)}
									>
										Delete
									</Button>
								{/if}
							</div>
							<div class="flex items-center gap-2">
								<Button
									onClick={() => setFolderContent(folderOriginal)}
									variant="subtle"
									unifiedSize="sm"
									startIcon={{ icon: RotateCcw }}
									disabled={saving || !folderChanged}
								>
									Reset
								</Button>
								<Button
									onClick={saveFolder}
									variant="accent"
									unifiedSize="sm"
									startIcon={{ icon: detail?.kind === 'new' ? Plus : Pencil }}
									disabled={saving ||
										!folderChanged ||
										(detail?.kind === 'new' && (!newPath || !!newPathError))}
								>
									{detail?.kind === 'new' ? 'Add instructions' : 'Save instructions'}
								</Button>
							</div>
						</div>
					</div>
				</div>
			</Section>
		</div>
	</div>
{/snippet}

<ConfirmationModal
	open={toDelete !== undefined}
	title="Delete folder instructions"
	confirmationText="Delete"
	onConfirmed={async () => {
		const row = toDelete
		toDelete = undefined
		if (row) await removeFolder(row)
	}}
	onCanceled={() => (toDelete = undefined)}
>
	<span class="text-xs text-primary">
		This deletes the resource at <span class="font-semibold">{toDelete?.path}</span>, so the
		assistant stops following it for everyone.
	</span>
</ConfirmationModal>
