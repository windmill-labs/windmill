<script lang="ts">
	import { onMount } from 'svelte'
	import { createDropdownMenu, melt } from '@melt-ui/svelte'
	import Button from '../common/button/Button.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Toggle from '../Toggle.svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import SettingCard from '../instanceSettings/SettingCard.svelte'
	import autosize from '$lib/autosize'
	import { conditionalMelt } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import { buildSkillMd, parseAndValidateSkill, parseSkillMd, type SkillUpload } from './aiSkills'
	import { ChevronDown, ClipboardPaste, Eye, FolderUp, Pencil, Plus, Trash2 } from 'lucide-svelte'

	type SkillListItem = { name: string; description: string }

	// `<root>/<skill>/SKILL.md` is 3 path segments; SKILL.md files nested deeper
	// are likely vendored/incidental and are skipped so importing a parent dir
	// doesn't sweep in unrelated skills.
	const MAX_SKILL_DEPTH = 3
	const MAX_SKILLS_PER_IMPORT = 50
	const MAX_SKILLS_PER_WORKSPACE = 100
	const SAMPLE_SKILL_PLACEHOLDER =
		'---\nname: my-skill\ndescription: what this skill helps with\n---\n\n# My skill\n\nInstructions for the assistant…'
	const menuItemClass =
		'w-full flex flex-row items-center gap-2.5 rounded-md px-2 py-1.5 text-left cursor-pointer transition-colors focus:outline-none data-[highlighted]:bg-surface-hover hover:bg-surface-hover'

	let skills: SkillListItem[] = $state([])
	let uploading: boolean = $state(false)
	let pasteContent: string = $state('')
	// The content the modal opened with, so Save can be gated on unsaved changes.
	let originalContent: string = $state('')
	let pasteModalOpen: boolean = $state(false)
	// Set while the paste modal is editing an existing skill; holds the skill's
	// name before edits so a rename can delete the old entry after save.
	let editingOriginalName: string | undefined = $state(undefined)
	let dirInput: HTMLInputElement | undefined = $state(undefined)
	let toDelete: string | undefined = $state(undefined)
	let pendingImport: SkillUpload[] | undefined = $state(undefined)
	let pendingSkipped: string[] = $state([])
	// Per-conflict overwrite choice for a folder import, keyed by skill name.
	let overwriteChoices: Record<string, boolean> = $state({})
	// The skill detail modal opens in read mode with rendered markdown; a header
	// toggle flips it to raw SKILL.md editing.
	let detailMode: 'view' | 'edit' = $state('view')
	let listRequestId = 0

	let existingNames = $derived(new Set(skills.map((s) => s.name)))
	let pendingConflicts = $derived(
		(pendingImport ?? ([] as SkillUpload[])).filter((s) => existingNames.has(s.name))
	)
	let pendingNew = $derived(
		(pendingImport ?? ([] as SkillUpload[])).filter((s) => !existingNames.has(s.name))
	)
	// Parsed view of the modal's raw content, for rendering the skill in read mode.
	let viewParsed = $derived(parseSkillMd(pasteContent))
	let isDirty = $derived(pasteContent !== originalContent)
	// Validate through the shared schema; surfaced inline so Save can be gated
	// without a toast.
	let pasteResult = $derived(parseAndValidateSkill(pasteContent))
	let pasteError = $derived('error' in pasteResult ? pasteResult.error : undefined)

	// Reset edit mode whenever the paste modal closes so a later "Paste a skill"
	// opens a blank creation form.
	$effect(() => {
		if (!pasteModalOpen) editingOriginalName = undefined
	})

	// melt dropdown for the "+ Add skills" button: arrow-key nav, outside/escape
	// close and focus management come for free.
	const {
		elements: { trigger: addMenuTrigger, menu: addMenu, item: addMenuItem },
		states: { open: addMenuOpen }
	} = createDropdownMenu({
		positioning: { placement: 'bottom-end', gutter: 4, fitViewport: true },
		loop: true,
		forceVisible: true
	})

	// attach the menu trigger to the design-system <Button>'s DOM node so it keeps
	// its styling — melt element stores are callable on a node like `use:melt`.
	let addTriggerEl: HTMLButtonElement | HTMLAnchorElement | undefined = $state(undefined)
	$effect(() => {
		const el = addTriggerEl
		if (!el) return
		const applied = conditionalMelt(el, addMenuTrigger as any) as { destroy?: () => void }
		return applied?.destroy
	})

	async function loadList(workspace: string | undefined) {
		const requestId = ++listRequestId
		if (!workspace) {
			skills = []
			return
		}
		try {
			const loaded = await WorkspaceService.listAiSkills({ workspace })
			if (requestId === listRequestId && workspace === $workspaceStore) {
				skills = loaded
			}
		} catch (e) {
			if (requestId === listRequestId && workspace === $workspaceStore) {
				sendUserToast(`Failed to load skills: ${e}`, true)
			}
		}
	}

	/**
	 * Turn a map of `relativePath -> content` (from an imported folder) into skills.
	 * A skill is any `SKILL.md`; its id is the name of the folder holding it.
	 */
	function collectSkills(files: Record<string, string>): {
		skills: SkillUpload[]
		skipped: string[]
	} {
		const collected: SkillUpload[] = []
		const skipped: string[] = []
		for (const [path, content] of Object.entries(files)) {
			const segments = path.split('/')
			if (segments[segments.length - 1]?.toLowerCase() !== 'skill.md') continue
			const folderName = segments.length >= 2 ? segments[segments.length - 2] : ''
			const result = parseAndValidateSkill(content, folderName)
			if ('error' in result) {
				skipped.push(`${folderName || path} (${result.error})`)
			} else {
				collected.push(result.skill)
			}
		}
		return { skills: collected, skipped }
	}

	async function uploadSkills(parsed: SkillUpload[], skipped: string[] = []) {
		const workspace = $workspaceStore
		if (!workspace || parsed.length === 0) {
			sendUserToast(
				`No valid skill found.${skipped.length ? ` Skipped: ${skipped.join(', ')}` : ''}`,
				true
			)
			return false
		}
		if (parsed.length > MAX_SKILLS_PER_IMPORT) {
			sendUserToast(`Cannot add more than ${MAX_SKILLS_PER_IMPORT} skills at a time.`, true)
			return false
		}
		// Uploads upsert, so only names not already stored count toward the cap.
		const newCount = parsed.filter((s) => !existingNames.has(s.name)).length
		if (skills.length + newCount > MAX_SKILLS_PER_WORKSPACE) {
			sendUserToast(`This workspace can store at most ${MAX_SKILLS_PER_WORKSPACE} skills.`, true)
			return false
		}
		uploading = true
		try {
			await WorkspaceService.uploadAiSkills({
				workspace,
				requestBody: { skills: parsed }
			})
			let message = `Added ${parsed.length} skill(s)`
			if (skipped.length) message += `; skipped ${skipped.length}: ${skipped.join(', ')}`
			sendUserToast(message)
			await loadList(workspace)
			return true
		} catch (e) {
			sendUserToast(`Failed to add skills: ${e}`, true)
			return false
		} finally {
			uploading = false
		}
	}

	function openPaste() {
		editingOriginalName = undefined
		pasteContent = ''
		originalContent = ''
		detailMode = 'edit'
		pasteModalOpen = true
	}

	async function openSkill(name: string, mode: 'view' | 'edit') {
		const workspace = $workspaceStore
		if (!workspace) return
		try {
			const skill = await WorkspaceService.getAiSkill({ workspace, name })
			pasteContent = buildSkillMd(skill)
			originalContent = pasteContent
			editingOriginalName = name
			detailMode = mode
			pasteModalOpen = true
		} catch (e) {
			sendUserToast(`Failed to load skill: ${e}`, true)
		}
	}

	async function submitPastedSkill() {
		// Guarded by `pasteError` disabling the button; bail defensively if reached.
		if (!('skill' in pasteResult)) return
		const parsed = pasteResult.skill
		const renamedFrom = editingOriginalName
		if (await uploadSkills([parsed])) {
			// A rename saves under the new name; drop the old entry so it doesn't linger.
			if (renamedFrom && renamedFrom !== parsed.name) {
				await deleteSkill(renamedFrom, { silent: true })
			}
			pasteContent = ''
			pasteModalOpen = false
		}
	}

	/**
	 * Filter a folder's files down to in-depth SKILL.md, read them, and stage the
	 * result for confirmation.
	 */
	async function processFolderFiles(files: File[]) {
		// Pick SKILL.md files within the depth limit BEFORE reading any content,
		// so a huge tree never gets read in full.
		const skipped: string[] = []
		const eligible: File[] = []
		for (const f of files) {
			const path = f.webkitRelativePath || f.name
			const segments = path.split('/')
			if (segments[segments.length - 1]?.toLowerCase() !== 'skill.md') continue
			if (segments.length > MAX_SKILL_DEPTH) {
				skipped.push(`${path} (nested deeper than ${MAX_SKILL_DEPTH} folder levels)`)
				continue
			}
			eligible.push(f)
		}

		if (eligible.length === 0) {
			sendUserToast(
				`No SKILL.md found within ${MAX_SKILL_DEPTH} folder levels.${
					skipped.length ? ` Skipped ${skipped.length} deeper file(s).` : ''
				}`,
				true
			)
			return
		}
		if (eligible.length > MAX_SKILLS_PER_IMPORT) {
			sendUserToast(
				`Found ${eligible.length} skills in this folder; imports are limited to ${MAX_SKILLS_PER_IMPORT} at a time.`,
				true
			)
			return
		}

		const map: Record<string, string> = {}
		for (const f of eligible) {
			map[f.webkitRelativePath || f.name] = await f.text()
		}
		const { skills: parsed, skipped: parseSkipped } = collectSkills(map)
		const allSkipped = [...skipped, ...parseSkipped]
		if (parsed.length === 0) {
			sendUserToast(
				`No valid skill found.${allSkipped.length ? ` Skipped: ${allSkipped.join(', ')}` : ''}`,
				true
			)
			return
		}
		// Confirm before writing — the import can pull in several skills at once,
		// and any that collide with existing skills default to overwrite.
		pendingSkipped = allSkipped
		overwriteChoices = Object.fromEntries(
			parsed.filter((s) => existingNames.has(s.name)).map((s) => [s.name, true])
		)
		pendingImport = parsed
	}

	async function onDirSelected(event: Event) {
		const target = event.target as HTMLInputElement
		const files = Array.from(target.files ?? [])
		// Reset early so re-selecting the same folder re-fires `change`.
		if (dirInput) dirInput.value = ''
		await processFolderFiles(files)
	}

	async function deleteSkill(name: string, opts: { silent?: boolean } = {}) {
		const workspace = $workspaceStore
		if (!workspace) return
		try {
			await WorkspaceService.deleteAiSkill({ workspace, name })
			if (!opts.silent) sendUserToast(`Deleted skill ${name}`)
			await loadList(workspace)
		} catch (e) {
			sendUserToast(`Failed to delete skill: ${e}`, true)
		}
	}

	onMount(() => {
		return workspaceStore.subscribe((workspace) => {
			toDelete = undefined
			pendingImport = undefined
			pendingSkipped = []
			overwriteChoices = {}
			void loadList(workspace)
		})
	})
</script>

{#snippet pasteZone()}
	<textarea
		bind:value={pasteContent}
		placeholder={SAMPLE_SKILL_PLACEHOLDER}
		class="w-full min-h-24 p-2 border border-gray-200 dark:border-gray-700 rounded-md bg-surface text-primary font-mono text-xs resize-y"
		rows="5"
		use:autosize
	></textarea>
	{#if pasteContent.trim()}
		<div class="flex items-center justify-between gap-2 mt-2">
			<span class="text-2xs text-red-500 min-w-0">{isDirty && pasteError ? pasteError : ''}</span>
			<Button
				onclick={submitPastedSkill}
				variant="accent"
				unifiedSize="sm"
				startIcon={{ icon: editingOriginalName ? Pencil : Plus }}
				disabled={uploading || !isDirty || !!pasteError}
			>
				{editingOriginalName ? 'Save skill' : 'Add skill'}
			</Button>
		</div>
	{/if}
{/snippet}

<SettingCard
	label="Custom skills"
	description="Add your own skills to the AI Chat. The expected format is the same as Claude or Codex."
>
	{#snippet headerAction()}
		<Button
			bind:element={addTriggerEl}
			{...$addMenuTrigger}
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: Plus }}
			endIcon={{ icon: ChevronDown }}
			disabled={uploading}
		>
			Add skills
		</Button>
		{#if $addMenuOpen}
			<div
				use:melt={$addMenu}
				class="z-[6000] flex flex-col gap-0.5 p-1 w-64 rounded-lg border border-gray-200 dark:border-gray-700 bg-surface shadow-xl focus:outline-none"
			>
				<button use:melt={$addMenuItem} class={menuItemClass} onclick={() => dirInput?.click()}>
					<FolderUp size={16} class="shrink-0 text-tertiary" />
					<span class="text-xs font-medium text-primary">Import a folder of skills</span>
				</button>
				<button use:melt={$addMenuItem} class={menuItemClass} onclick={openPaste}>
					<ClipboardPaste size={16} class="shrink-0 text-tertiary" />
					<span class="text-xs font-medium text-primary">Paste a skill</span>
				</button>
			</div>
		{/if}
	{/snippet}

	<div class="flex flex-col gap-3 pt-1">
		{#if skills.length === 0}
			<div class="rounded-md border border-dashed px-3 py-6 text-center text-xs text-secondary">
				No custom skills yet
			</div>
		{:else}
			<div class="rounded-md border divide-y">
				{#each skills as skill (skill.name)}
					<div class="flex items-center justify-between gap-4 px-3 py-2">
						<div class="min-w-0">
							<div class="text-xs font-mono truncate">{skill.name}</div>
							<div class="text-2xs text-secondary truncate">{skill.description}</div>
							<button
								class="text-2xs text-secondary hover:text-primary mt-0.5"
								onclick={() => openSkill(skill.name, 'view')}
							>
								Show more
							</button>
						</div>
						<DropdownV2
							size="sm"
							items={[
								{
									displayName: 'Edit',
									icon: Pencil,
									disabled: uploading,
									action: () => openSkill(skill.name, 'edit')
								},
								{
									displayName: 'Delete',
									icon: Trash2,
									type: 'delete',
									action: () => (toDelete = skill.name)
								}
							]}
						/>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</SettingCard>

<!-- Hidden folder picker fired by the dropdown's "Import a folder of skills". -->
<input
	bind:this={dirInput}
	type="file"
	style="display: none;"
	onchange={onDirSelected}
	{...{ webkitdirectory: true, directory: true }}
/>

<Modal2
	title={editingOriginalName ?? 'Paste a skill'}
	bind:isOpen={pasteModalOpen}
	fixedWidth="md"
	fixedHeight="adaptive"
>
	{#snippet headerRight()}
		{#if editingOriginalName}
			<ToggleButtonGroup bind:selected={detailMode}>
				{#snippet children({ item })}
					<ToggleButton value="view" label="View" icon={Eye} {item} small />
					<ToggleButton value="edit" label="Edit" icon={Pencil} {item} small />
				{/snippet}
			</ToggleButtonGroup>
		{/if}
	{/snippet}
	<div class="w-full flex flex-col">
		{#if detailMode === 'view'}
			<div class="w-full flex flex-col gap-3">
				{#if viewParsed.description}
					<p class="text-xs text-secondary">{viewParsed.description}</p>
				{/if}
				<div
					class="border rounded-md p-3 overflow-auto max-h-[60vh] prose prose-sm dark:prose-invert max-w-full leading-snug space-y-2 prose-ul:!pl-6
						prose-p:text-xs prose-li:text-xs prose-code:text-xs prose-pre:text-xs
						prose-code:break-words prose-a:break-words
						prose-headings:font-medium prose-headings:text-emphasis prose-headings:mt-3 prose-headings:mb-1
						prose-h1:text-sm prose-h2:text-xs prose-h3:text-xs prose-h4:text-xs prose-h5:text-xs prose-h6:text-xs
						prose-table:block prose-table:max-w-full prose-table:overflow-x-auto prose-table:text-xs"
				>
					<Markdown md={viewParsed.instructions} plugins={[gfmPlugin()]} />
				</div>
			</div>
		{:else}
			{@render pasteZone()}
		{/if}
	</div>
</Modal2>

<ConfirmationModal
	open={pendingImport !== undefined}
	title="Import skills"
	confirmationText="Import"
	onConfirmed={async () => {
		const toImport = [...pendingNew, ...pendingConflicts.filter((s) => overwriteChoices[s.name])]
		const skipped = pendingSkipped
		pendingImport = undefined
		pendingSkipped = []
		overwriteChoices = {}
		if (toImport.length) await uploadSkills(toImport, skipped)
		else sendUserToast('No skills imported.')
	}}
	onCanceled={() => {
		pendingImport = undefined
		pendingSkipped = []
		overwriteChoices = {}
	}}
>
	<div class="flex flex-col gap-3 text-xs">
		{#if pendingNew.length}
			<div>
				<span class="font-medium text-primary">Add {pendingNew.length} new skill(s):</span>
				<span class="font-mono text-secondary">{pendingNew.map((s) => s.name).join(', ')}</span>
			</div>
		{/if}
		{#if pendingConflicts.length}
			<div class="flex flex-col gap-1.5">
				<span class="font-medium text-primary">
					{pendingConflicts.length} skill(s) already exist — choose which to overwrite:
				</span>
				<div class="rounded-md border divide-y">
					{#each pendingConflicts as conflict (conflict.name)}
						<div class="flex items-center justify-between gap-4 px-3 py-2">
							<span class="font-mono truncate">{conflict.name}</span>
							<Toggle
								bind:checked={overwriteChoices[conflict.name]}
								size="xs"
								options={{ right: 'Overwrite' }}
							/>
						</div>
					{/each}
				</div>
			</div>
		{/if}
		{#if pendingSkipped.length}
			<span class="text-secondary">{pendingSkipped.length} file(s) will be skipped.</span>
		{/if}
	</div>
</ConfirmationModal>

<ConfirmationModal
	open={toDelete !== undefined}
	title="Delete skill"
	confirmationText="Delete"
	onConfirmed={async () => {
		const name = toDelete
		toDelete = undefined
		if (name) await deleteSkill(name)
	}}
	onCanceled={() => (toDelete = undefined)}
>
	<span>
		Delete the skill <code>{toDelete}</code>? The AI chat will no longer be able to use it.
	</span>
</ConfirmationModal>
