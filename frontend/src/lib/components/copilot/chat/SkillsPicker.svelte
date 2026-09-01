<script lang="ts">
	import { Alert, Button, Drawer } from '$lib/components/common'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Path from '$lib/components/Path.svelte'
	import FileInput from '$lib/components/common/fileInput/FileInput.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { markdownProse } from '$lib/components/markdownProse'
	import { workspaceStore, userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import type { Item } from '$lib/utils'
	import { untrack } from 'svelte'
	import {
		BookOpen,
		ClipboardPaste,
		Eye,
		List,
		Pencil,
		Plus,
		RotateCcw,
		Trash2
	} from 'lucide-svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { isSkillEnabled, setSkillEnabled } from './skills/enabledSkills'
	import {
		ambiguousSkillNames,
		deleteSkillResource,
		listSkillResources,
		readSkillBody,
		saveSkillResource,
		updateSkillResource,
		type SkillResource
	} from './skills/skillResources'
	import {
		buildSkillMd,
		parseAndValidateSkill,
		parseSkillMd,
		type SkillUpload
	} from './skills/skillMd'

	const aiChatManager = getAiChatManager()

	// A session chat operates on its own (possibly forked) workspace without
	// switching `workspaceStore`, and that is the workspace the chat reads the
	// enabled set under. Key everything here the same way or a toggle lands under
	// a key nothing reads.
	//
	// `operatingWorkspace` is a plain getter over untracked state, so the store is
	// read unconditionally rather than behind `??`: short-circuiting it would leave
	// this derived with no dependency at all, frozen on the workspace it first saw.
	let ws = $derived.by(() => {
		const active = $workspaceStore
		return aiChatManager.operatingWorkspace ?? active!
	})

	// A session whose fork is still staged has no workspace of its own yet, so `ws`
	// resolves to the PARENT. Authoring through the picker would then edit the live
	// parent, and a toggle would be stored under it and quietly stop applying the
	// moment the first send commits the fork. Read at use, not once: the fork
	// commits mid-session.
	function pendingForkParent(): string | undefined {
		return aiChatManager.sessionContextResolver?.()?.pendingForkOf
	}
	let forkPending = $state(false)
	function refreshForkPending() {
		forkPending = pendingForkParent() !== undefined
	}
	/** Guards every mutating action. Returns true when the caller must not proceed. */
	function blockedByPendingFork(): boolean {
		const parent = pendingForkParent()
		if (parent === undefined) return false
		refreshForkPending()
		sendUserToast(
			`This session has not created its workspace yet, so a skill would be written to "${parent}" instead. Send a message first.`,
			true
		)
		return true
	}

	// `<root>/<skill>/SKILL.md` is 3 path segments; SKILL.md files nested deeper
	// are likely vendored/incidental and are skipped so importing a parent dir
	// doesn't sweep in unrelated skills.
	const MAX_SKILL_DEPTH = 3
	const MAX_SKILLS_PER_IMPORT = 50
	/** Stands in for the name when only the description and body are being checked. */
	const VALID_NAME_PLACEHOLDER = 'placeholder'
	// A new skill opens on this as real, editable text rather than ghost placeholder
	// text: the format is the point of the sample, and a SKILL.md is easier to adapt
	// than to recall. `name` seeds the path above until the user edits the path
	// themselves, so renaming here renames the skill.
	const SKILL_TEMPLATE = `---
name: my-skill
description: What this skill covers, and when the assistant should reach for it
---

# My skill

What the assistant should do when this skill applies.

## Steps

1. First thing to do.
2. Second thing to do.
`

	type Row = SkillResource & { enabled: boolean }

	let drawer: Drawer | undefined = $state(undefined)
	let skills = $state<Row[]>([])
	let loading = $state(false)
	let loadError = $state<string | undefined>(undefined)
	let saving = $state(false)
	let toDelete: Row | undefined = $state(undefined)
	let importFiles = $state<File[] | undefined>(undefined)

	// Paste/edit modal. `editing` is the row being edited (undefined while
	// creating), held so a path change can be applied as a move.
	let editorOpen = $state(false)
	let editing = $state<Row | undefined>(undefined)
	let content = $state('')
	let originalContent = $state('')
	let path = $state('')
	let pathError = $state('')
	// Set by Path once the user edits the path themselves, which is what stops the
	// frontmatter from overwriting their choice below.
	let pathDirty = $state(false)
	let detailMode: 'view' | 'edit' = $state('view')

	// Staged folder import, confirmed before anything is written.
	let pendingImport: SkillUpload[] | undefined = $state(undefined)
	let pendingSkipped: string[] = $state([])
	let overwriteChoices: Record<string, boolean> = $state({})

	let ambiguous = $derived(ambiguousSkillNames(skills))
	let parsed = $derived(parseSkillMd(content))
	// The Path field is what names the skill, and Path validates it. The frontmatter
	// `name` only seeds that field and is never persisted, so validating it here
	// would block saving a skill whose path is legal but whose name is not — resource
	// paths admit `_` and uppercase, SKILL.md names do not. The folder importer keeps
	// validating, because there the name really does become the path segment.
	let validated = $derived(parseAndValidateSkill(content, VALID_NAME_PLACEHOLDER))
	let contentError = $derived('error' in validated ? validated.error : undefined)
	// Measured against what the modal opened with, which for a new skill is the
	// sample. Saving it untouched would store a skill called "my-skill", so an
	// unedited body is not something to save; Reset restores exactly this baseline.
	let contentChanged = $derived(content !== originalContent)
	let pathChanged = $derived(!!editing && path !== editing.path)
	let canSave = $derived(
		!saving && !contentError && !!path && !pathError && (contentChanged || pathChanged)
	)
	// Keyed by the path the import would write to, not by bare name: a `deploy` that
	// exists only in someone else's folder is not something this import overwrites,
	// and offering it as a conflict would ask the user about a collision that isn't.
	let existingPaths = $derived(new Set(skills.map((s) => s.path)))
	let importTargets = $derived(
		(pendingImport ?? ([] as SkillUpload[])).map((s) => ({
			skill: s,
			path: `${defaultOwner()}/${s.name}`
		}))
	)
	let pendingConflicts = $derived(
		importTargets.filter((t) => existingPaths.has(t.path)).map((t) => t.skill)
	)
	let pendingNew = $derived(
		importTargets.filter((t) => !existingPaths.has(t.path)).map((t) => t.skill)
	)

	// Rows describe one workspace. A switch while the drawer is open must not leave
	// A's rows on screen while the actions below target B: same path, different
	// skill, and delete would remove the wrong one. Dropping them is all this does:
	// this component mounts with the chat toolbar, so loading here would list
	// resources for every user who never opens the menu. The two entry points load
	// what they need.
	let loadSeq = 0
	$effect(() => {
		const target = ws
		untrack(() => {
			loadSeq++
			skills = []
			toDelete = undefined
			pendingImport = undefined
			// The editor holds one workspace's skill but saves to whatever `ws` is by
			// then, so a switch mid-edit would write A's body into B at the same path.
			// Closing it is the honest outcome: there is no version of that save the
			// user asked for.
			editorOpen = false
			editing = undefined
			// A drawer already on screen is neither entry point, and would sit there
			// reporting that the new workspace has no skills.
			if (drawer?.isOpen()) void loadSkills(target)
		})
	})

	async function loadSkills(target = ws) {
		if (!target) return
		refreshForkPending()
		const seq = ++loadSeq
		loading = true
		loadError = undefined
		try {
			const found = await listSkillResources(target, $userStore ?? undefined)
			// Newest-request-wins is not enough on its own: an action started in A and
			// finishing after a switch to B holds the newest sequence, and would put
			// A's rows on screen under B — where the next row action would edit or
			// delete that path in B. The workspace has to still be the one asked for.
			if (seq !== loadSeq || target !== ws) return
			skills = found.map((s) => ({ ...s, enabled: isSkillEnabled(target, s.path) }))
		} catch (e) {
			if (seq !== loadSeq || target !== ws) return
			// Without this the drawer would render the empty state, which reads as
			// "this workspace has no skills" rather than "we could not load them".
			loadError = e.body ?? e.message
		} finally {
			if (seq === loadSeq) loading = false
		}
	}

	export async function open() {
		drawer?.openDrawer()
		await loadSkills()
	}

	// A menu is a shortcut, not a directory: past this many the list stops being
	// scannable, so the rest are reached through the drawer rather than dropped.
	const MAX_MENU_SKILLS = 8

	/** Rows for the chat's "+" menu: one per skill, checked when it is on, then
	 * the way to manage them. Loaded on open so the checks are current. */
	export async function menuItems(closeMenu?: () => void): Promise<Item[]> {
		// The menu opens on what is already known and refreshes behind it: waiting on
		// a round trip would stall the whole `+` menu, attachments included.
		if (skills.length === 0) {
			await loadSkills()
		} else {
			void loadSkills()
		}
		// Enabled first: those are the ones a quick visit is most likely about.
		const ordered = [...skills].sort(
			(a, b) => Number(b.enabled) - Number(a.enabled) || a.path.localeCompare(b.path)
		)
		const shown = ordered.slice(0, MAX_MENU_SKILLS)
		return [
			...shown.map(({ path: p, name }) => ({
				// Ambiguous names are shown by path — two rows reading `deploy` would
				// leave the choice between them to chance.
				displayName: ambiguous.has(name) ? p : name,
				icon: BookOpen,
				// Getters, not snapshots: the menu stays open across a click, and it has
				// to read through the live list rather than the row captured here, since
				// a reload replaces every row object and a getter bound to the old one
				// would go on reporting the state it was built with.
				get toggle() {
					return row(p)?.enabled ?? false
				},
				action: () => toggle(p, !row(p)?.enabled)
			})),
			...(ordered.length > shown.length
				? [
						{
							displayName: `Show all ${ordered.length}`,
							icon: List,
							action: () => {
								closeMenu?.()
								void open()
							}
						}
					]
				: []),
			{
				displayName: skills.length > 0 ? 'Manage skills' : 'Add a skill',
				icon: Plus,
				separatorTop: skills.length > 0,
				action: () => {
					closeMenu?.()
					void open()
				}
			}
		]
	}

	function row(p: string) {
		return skills.find((s) => s.path === p)
	}

	async function toggle(p: string, enabled: boolean) {
		if (blockedByPendingFork()) return
		if (!setSkillEnabled(ws, p, enabled)) {
			sendUserToast('Could not save the selection for this account.', true)
			return
		}
		const skill = skills.find((s) => s.path === p)
		if (skill) skill.enabled = enabled
		// Whether people select skills at all. Never the skill itself: a path is
		// workspace-authored text.
		logFeatureUsage('ai_session', 'skill_toggle', { key: enabled ? 'on' : 'off', workspace: ws })
		// The prompt lists exactly the enabled skills, so it has to be rebuilt
		// before the next message rather than on the next mode change.
		await aiChatManager.refreshGlobalSkills(ws)
	}

	/** Personal folder the folder import writes into. The username is not always a
	 * legal path segment — a superadmin who is not a member of the workspace gets
	 * their email back from `whoami` — and `resource.path` is CHECK-constrained, so
	 * it is narrowed the same way Path.svelte narrows it. */
	function defaultOwner() {
		const username = $userStore?.username ?? 'user'
		const narrowed = username.includes('@')
			? username.split('@')[0].replace(/[^a-zA-Z0-9_]/g, '')
			: username
		// Narrowing can empty the string outright (an all-punctuation local part),
		// and `u//name` fails the path CHECK with nothing to explain it.
		return `u/${narrowed || 'user'}`
	}

	function openCreate() {
		if (blockedByPendingFork()) return
		editing = undefined
		content = SKILL_TEMPLATE
		originalContent = SKILL_TEMPLATE
		path = ''
		pathDirty = false
		detailMode = 'edit'
		editorOpen = true
	}

	/** Back to what the modal opened with: the sample for a new skill, the saved
	 * body for one being edited. */
	function resetContent() {
		content = originalContent
	}

	async function openSkill(skill: Row, mode: 'view' | 'edit') {
		// Pinned across the await: the workspace can change while the body loads, and
		// closing the editor then would not stop this from reopening it with the old
		// workspace's content — which submitEditor would save into the new one.
		const source = ws
		try {
			const instructions = await readSkillBody(source, skill.path)
			if (source !== ws) return
			content = buildSkillMd({
				name: skill.name,
				description: skill.description,
				instructions
			})
			originalContent = content
			editing = skill
			path = skill.path
			pathDirty = false
			detailMode = mode
			editorOpen = true
		} catch (e) {
			sendUserToast(`Failed to load skill: ${e.body ?? e.message}`, true)
		}
	}

	// Path opens on a generated name, so the frontmatter `name` has to replace that
	// last segment rather than fill a blank. Its owner is left alone (Path picks up
	// where the user last created something), and the whole thing stops as soon as
	// they edit the path themselves.
	//
	// `path` is a dependency, not just a value read: Path settles it asynchronously,
	// after the template's name is already parsed, so an effect watching only the
	// content would run once against an empty path and never again. Writing back the
	// value it already holds is what would loop, hence the equality guard.
	$effect(() => {
		const suggested = parsed.name
		const current = path
		untrack(() => {
			if (!editorOpen || editing || pathDirty || !suggested) return
			const owner = current.split('/').slice(0, -1).join('/')
			if (!owner) return
			const next = `${owner}/${suggested}`
			if (next !== current) path = next
		})
	})

	async function submitEditor() {
		if (!('skill' in validated) || !path || pathError) return
		if (blockedByPendingFork()) return
		const target = ws
		saving = true
		try {
			if (editing) {
				await updateSkillResource(
					target,
					editing.path,
					path,
					validated.skill.description,
					validated.skill.instructions
				)
				// A move leaves the old path selected but gone; carry the choice over
				// so an edit that renames does not silently switch the skill off.
				if (editing.path !== path && isSkillEnabled(target, editing.path)) {
					setSkillEnabled(target, editing.path, false)
					setSkillEnabled(target, path, true)
				}
			} else {
				await saveSkillResource(
					target,
					path,
					validated.skill.description,
					validated.skill.instructions
				)
				// Authoring a skill is the act of choosing it.
				setSkillEnabled(target, path, true)
			}
			editorOpen = false
			await refresh(target)
			sendUserToast(editing ? `Saved ${path}` : `Added ${path}`)
		} catch (e) {
			sendUserToast(`Failed to save skill: ${e.body ?? e.message}`, true)
		} finally {
			saving = false
		}
	}

	async function remove(skill: Row) {
		if (blockedByPendingFork()) return
		const target = ws
		try {
			await deleteSkillResource(target, skill.path)
			// A later skill at this path is a different one; it must be turned on
			// deliberately rather than inherit this one's selection.
			setSkillEnabled(target, skill.path, false)
			sendUserToast(`Deleted ${skill.path}`)
			await refresh(target)
		} catch (e) {
			sendUserToast(`Failed to delete skill: ${e.body ?? e.message}`, true)
		}
	}

	async function refresh(target = ws) {
		await loadSkills(target)
		// Same reason: refreshing the chat with a workspace it has since left would
		// advertise A's skills to a session now acting on B.
		if (target !== ws) return
		await aiChatManager.refreshGlobalSkills(target)
	}

	/** Where the file sat in the chosen folder. Clicking through sets
	 * `webkitRelativePath`; dropping sets `path`, which the picker's tree walk fills. */
	function relativePathOf(f: File & { path?: string }): string {
		return f.webkitRelativePath || f.path || f.name
	}

	/**
	 * Filter a folder's files down to in-depth SKILL.md, read them, and stage the
	 * result for confirmation.
	 */
	async function processFolderFiles(files: File[]) {
		if (blockedByPendingFork()) return
		// Pick SKILL.md files within the depth limit BEFORE reading any content,
		// so a huge tree never gets read in full.
		const skipped: string[] = []
		const eligible: File[] = []
		for (const f of files) {
			const filePath = relativePathOf(f)
			const segments = filePath.split('/')
			if (segments[segments.length - 1]?.toLowerCase() !== 'skill.md') continue
			if (segments.length > MAX_SKILL_DEPTH) {
				skipped.push(`${filePath} (nested deeper than ${MAX_SKILL_DEPTH} folder levels)`)
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

		const collected: SkillUpload[] = []
		const parseSkipped: string[] = []
		for (const f of eligible) {
			const filePath = relativePathOf(f)
			const segments = filePath.split('/')
			// The skill's id is the folder holding its SKILL.md, which is what
			// becomes the resource path's name segment.
			const folderName = segments.length >= 2 ? segments[segments.length - 2] : ''
			const result = parseAndValidateSkill(await f.text(), folderName)
			if ('error' in result) {
				parseSkipped.push(`${folderName || filePath} (${result.error})`)
				continue
			}
			// Two folders under different parents can share a leaf name, and both
			// would target one path — the second silently replacing the first.
			if (collected.some((s) => s.name === result.skill.name)) {
				parseSkipped.push(
					`${filePath} (another skill in this folder is already named ${result.skill.name})`
				)
				continue
			}
			collected.push(result.skill)
		}

		const allSkipped = [...skipped, ...parseSkipped]
		if (collected.length === 0) {
			sendUserToast(
				`No valid skill found.${allSkipped.length ? ` Skipped: ${allSkipped.join(', ')}` : ''}`,
				true
			)
			return
		}
		// Confirm before writing — the import can pull in several skills at once,
		// and any that collide with existing skills default to overwrite.
		pendingSkipped = allSkipped
		const owner = defaultOwner()
		overwriteChoices = Object.fromEntries(
			collected.filter((s) => existingPaths.has(`${owner}/${s.name}`)).map((s) => [s.name, true])
		)
		pendingImport = collected
	}

	async function onDirSelected(event: CustomEvent<File[] | undefined>) {
		const files = event.detail ?? []
		// Clearing lets the same folder be chosen again — the component keeps the
		// last selection on screen otherwise, and re-picking would look inert.
		importFiles = undefined
		if (files.length) await processFolderFiles(files)
	}

	/** `overwrite` is granted only for destinations the confirmation listed as an
	 * existing skill. Everything else is created, never upserted: a path the user
	 * was told was free may hold an unrelated resource — a credential, say — and an
	 * upsert would replace its value and type with a skill. Losing the import of a
	 * name is recoverable; losing what was there is not. */
	async function importSkills(
		toImport: { skill: SkillUpload; overwrite: boolean }[],
		skipped: string[]
	) {
		const target = ws
		const owner = defaultOwner()
		saving = true
		let written = 0
		const failed: string[] = []
		try {
			for (const { skill, overwrite } of toImport) {
				const dest = `${owner}/${skill.name}`
				try {
					await saveSkillResource(target, dest, skill.description, skill.instructions, {
						overwrite
					})
					setSkillEnabled(target, dest, true)
					written++
				} catch (e) {
					failed.push(`${skill.name} (${e.body ?? e.message})`)
				}
			}
			let message = `Added ${written} skill(s) under ${owner}`
			if (skipped.length) message += `; skipped ${skipped.length}`
			if (failed.length) message += `; failed: ${failed.join(', ')}`
			sendUserToast(message, failed.length > 0)
			await refresh(target)
		} finally {
			saving = false
		}
	}
</script>

<Drawer bind:this={drawer} size="700px">
	<DrawerContent
		title="Skills"
		on:close={() => drawer?.closeDrawer()}
		tooltip="Reusable instruction sets for this chat, stored as ai_skill resources. Turning one on is personal to you and to this workspace — the assistant only sees the ones you selected."
	>
		{#snippet actions()}
			<Button
				variant="accent"
				unifiedSize="sm"
				startIcon={{ icon: ClipboardPaste }}
				disabled={saving || forkPending}
				onclick={openCreate}
			>
				New skill
			</Button>
		{/snippet}

		{#if forkPending}
			<Alert type="info" title="This session has no workspace yet" size="xs" class="mb-4">
				Skills are read-only until the first message creates this session's fork. Editing one now
				would change the parent workspace instead.
			</Alert>
		{/if}

		<FileInput
			disabled={forkPending}
			folderOnly
			bind:files={importFiles}
			on:change={onDirSelected}
			class="mb-4 !py-5"
			iconSize={20}
		>
			<span class="text-xs text-secondary">
				Drop a folder of <span class="font-mono">SKILL.md</span> files to import, or click to choose
				one
			</span>
		</FileInput>

		{#if loading}
			<div class="text-xs text-secondary p-4 text-center">Loading skills…</div>
		{:else if loadError}
			<div class="text-xs text-red-600 dark:text-red-400">
				Failed to load skills: {loadError}
			</div>
		{:else if skills.length === 0}
			<div class="rounded-md border border-dashed px-3 py-6 text-center text-xs text-secondary">
				No skills in this workspace yet. Paste a SKILL.md or import a folder of them.
			</div>
		{:else}
			<div class="flex flex-col divide-y border rounded-md bg-surface-tertiary">
				{#each skills as skill (skill.path)}
					<div class="flex items-center gap-3 px-4 py-3">
						<BookOpen size={16} class="shrink-0 text-tertiary" />
						<div class="min-w-0 grow">
							<div class="text-xs font-semibold text-emphasis truncate">
								{ambiguous.has(skill.name) ? skill.path : skill.name}
							</div>
							{#if skill.description}
								<div class="text-xs text-secondary truncate">{skill.description}</div>
							{/if}
						</div>
						<Toggle
							size="xs"
							checked={skill.enabled}
							on:change={async (e) => await toggle(skill.path, e.detail)}
						/>
						<DropdownV2
							size="sm"
							items={[
								{
									displayName: skill.canWrite ? 'Edit' : 'View',
									icon: skill.canWrite ? Pencil : Eye,
									action: () => openSkill(skill, skill.canWrite ? 'edit' : 'view')
								},
								{
									displayName: 'Delete',
									icon: Trash2,
									type: 'delete',
									disabled: !skill.canWrite,
									action: () => (toDelete = skill)
								}
							]}
						/>
					</div>
				{/each}
			</div>
		{/if}

		<ConfirmationModal
			open={toDelete !== undefined}
			title="Delete skill"
			confirmationText="Delete"
			onConfirmed={async () => {
				const skill = toDelete
				toDelete = undefined
				if (skill) await remove(skill)
			}}
			onCanceled={() => (toDelete = undefined)}
		>
			<span class="text-xs text-primary">
				This deletes the resource at <span class="font-semibold">{toDelete?.path}</span>, so
				everyone who selected it loses the skill.
			</span>
		</ConfirmationModal>

		<ConfirmationModal
			open={pendingImport !== undefined}
			title="Import skills"
			type="info"
			confirmationText="Import"
			onConfirmed={async () => {
				const toImport = [
					...pendingNew.map((skill) => ({ skill, overwrite: false })),
					...pendingConflicts
						.filter((s) => overwriteChoices[s.name])
						.map((skill) => ({ skill, overwrite: true }))
				]
				const skipped = pendingSkipped
				pendingImport = undefined
				pendingSkipped = []
				overwriteChoices = {}
				if (toImport.length) await importSkills(toImport, skipped)
				else sendUserToast('No skills imported.')
			}}
			onCanceled={() => {
				pendingImport = undefined
				pendingSkipped = []
				overwriteChoices = {}
			}}
		>
			<div class="flex flex-col gap-3 text-xs">
				<span class="text-secondary">
					Skills are added under <span class="font-mono">{defaultOwner()}</span>. Move one to a
					shared folder from the resources page to share it.
				</span>
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
	</DrawerContent>
</Drawer>

<Modal2
	title={editing ? editing.name : 'New skill'}
	bind:isOpen={editorOpen}
	fixedWidth="md"
	fixedHeight="adaptive"
>
	{#snippet headerRight()}
		{#if editing}
			<ToggleButtonGroup bind:selected={detailMode}>
				{#snippet children({ item })}
					<ToggleButton value="view" label="View" icon={Eye} {item} small />
					<ToggleButton
						value="edit"
						label="Edit"
						icon={Pencil}
						{item}
						small
						disabled={!editing?.canWrite}
					/>
				{/snippet}
			</ToggleButtonGroup>
		{/if}
	{/snippet}
	<div class="w-full flex flex-col gap-3">
		{#if detailMode === 'view'}
			{#if parsed.description}
				<p class="text-xs text-secondary">{parsed.description}</p>
			{/if}
			<div class="border rounded-md p-3 overflow-auto max-h-[60vh] space-y-2 {markdownProse.sm}">
				<Markdown md={parsed.instructions} plugins={[gfmPlugin()]} />
			</div>
		{:else}
			<Path
				bind:path
				bind:error={pathError}
				bind:dirty={pathDirty}
				initialPath={editing?.path ?? ''}
				namePlaceholder="skill"
				kind="resource"
				workspaceOverride={ws}
				autofocus={false}
			/>
			<!-- The same editor `/resources` gives these resources, so a skill reads the
			     same in both places. `fixedOverflowWidgets` off keeps its autocomplete
			     inside the modal instead of clipped behind it. -->
			<div class="border border-border-light rounded-md overflow-hidden">
				<SimpleEditor
					autoHeight
					lang="md"
					bind:code={content}
					fixedOverflowWidgets={false}
					class="min-h-24"
				/>
			</div>
			<div class="flex items-center justify-between gap-2">
				<span class="text-2xs text-red-500 min-w-0">{contentError ?? ''}</span>
				<div class="flex items-center gap-2">
					<Button
						onclick={resetContent}
						variant="subtle"
						unifiedSize="sm"
						startIcon={{ icon: RotateCcw }}
						disabled={saving || !contentChanged}
						title={editing ? 'Discard unsaved changes to this skill' : 'Restore the sample skill'}
					>
						Reset
					</Button>
					<Button
						onclick={submitEditor}
						variant="accent"
						unifiedSize="sm"
						startIcon={{ icon: editing ? Pencil : Plus }}
						disabled={!canSave}
					>
						{editing ? 'Save skill' : 'Add skill'}
					</Button>
				</div>
			</div>
		{/if}
	</div>
</Modal2>
