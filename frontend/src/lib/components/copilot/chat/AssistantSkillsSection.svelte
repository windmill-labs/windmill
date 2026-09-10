<!--
@component
The Skills section of the assistant settings modal: every ai_skill resource in the
operating workspace, each with the switch that decides whether this chat carries it,
and the actions that create, edit, import and delete them.
-->
<script lang="ts">
	import { Button, ListRow, Section } from '$lib/components/common'
	import EmptyState from '$lib/components/common/emptyState/EmptyState.svelte'
	import FileInput from '$lib/components/common/fileInput/FileInput.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import PagedContent from '$lib/components/common/modal/PagedContent.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { markdownProse } from '$lib/components/markdownProse'
	import { userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { tick, untrack } from 'svelte'
	import {
		ArrowLeft,
		BookOpen,
		ChevronDown,
		ChevronRight,
		Eye,
		Folder,
		FolderTree,
		FolderUp,
		Pencil,
		Plus,
		RotateCcw,
		Trash2,
		User
	} from 'lucide-svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { forgetSkill, isSkillEnabled, setSkillEnabled } from './skills/enabledSkills'
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
	import {
		buildSkillTree,
		countSkills,
		folderKey,
		nodeSkills,
		skillFolderPaths,
		skillKey,
		visibleEntries,
		type SkillTreeEntry,
		type SkillTreeNode
	} from './skills/skillTree'

	let {
		ws,
		active,
		count = $bindable(),
		blocksClose = $bindable()
	}: {
		/** The workspace the chat operates on, which is not always the one on screen. */
		ws: string
		/** Whether this is the panel on screen. Gates the editor page's build: it is
		 * worth paying for once someone is looking at skills, not on every open of a
		 * modal whose other three sections have nothing to do with them. */
		active: boolean
		/** Number of skills in the workspace, for the sidebar badge. */
		count: number
		/** True while this section is in the middle of something the modal must not
		 * close under: a dialog of its own, or the editor holding unsaved text. */
		blocksClose: boolean
	} = $props()

	const aiChatManager = getAiChatManager()

	// A session whose fork is still staged has no workspace of its own yet, so `ws`
	// resolves to the PARENT. Authoring here would then edit the live parent, and a
	// toggle would be stored under it and quietly stop applying the moment the first
	// send commits the fork. Read at use, not once: the fork commits mid-session.
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

	let skills = $state<Row[]>([])
	let loading = $state(false)
	let loadError = $state<string | undefined>(undefined)
	let listNotice = $state<string | undefined>(undefined)
	let saving = $state(false)
	let toDelete: Row | undefined = $state(undefined)
	let folderInput: FileInput | undefined = $state(undefined)
	let importFiles = $state<File[] | undefined>(undefined)
	let bodyEditor: SimpleEditor | undefined = $state(undefined)

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
	// Bumped for each visit to the editor. The editor page stays mounted once opened,
	// and Path settles its owner and name segments on mount alone: without a remount,
	// the second skill opened would keep the first one's fields, and a new skill would
	// sit on the empty path `openCreate` leaves behind.
	let editorSeq = $state(0)
	// 'edit' as the default is what the warm build of the editor page renders, and the
	// editor is the half worth building ahead of the click. Both entry points set this
	// before opening, so the default is never what a visit actually lands on.
	let detailMode: 'view' | 'edit' = $state('edit')

	// Staged folder import, confirmed before anything is written.
	let pendingImport: SkillUpload[] | undefined = $state(undefined)
	let pendingSkipped: string[] = $state([])
	let overwriteChoices: Record<string, boolean> = $state({})

	let ambiguous = $derived(ambiguousSkillNames(skills))
	// Skills in one folder are a list; spread over several, the folder is what tells
	// two of them apart, so they are grouped under it instead.
	let grouped = $derived(skillFolderPaths(skills).size > 1)
	let tree = $derived(buildSkillTree(skills))
	/** Folders the user closed, keyed by full prefix. Everything opens expanded: the
	 * point of this list is seeing what the assistant carries. */
	let collapsed = $state<Record<string, boolean>>({})
	/** The list top to bottom as it stands, for the keyboard to walk. Empty whenever
	 * the rows are not the thing on screen — a reload after a save or a delete shows
	 * the loading line, and a failed one shows the error, both over the rows this
	 * would otherwise still walk. Flat mode has no folders, so the same keys drive
	 * both layouts. */
	let entries: SkillTreeEntry<Row>[] = $derived(
		loading || loadError !== undefined
			? []
			: grouped
				? visibleEntries(tree, (path) => collapsed[path] === true)
				: skills.map((skill) => ({ kind: 'skill', key: skillKey(skill.path), skill }))
	)
	let highlightedKey = $state<string | undefined>(undefined)
	/** The keyboard is driving. Hover stops lighting rows while it is: two lit rows at
	 * once leaves no answer to which one Space acts on. */
	let keyboardActive = $state(false)
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

	$effect(() => {
		count = skills.length
	})
	// A row's overflow menu is portaled out of the modal, so a click on one of its
	// items is a click outside the modal. Tracking it here keeps the modal from
	// closing under the action the item is about to run.
	let rowMenuOpen = $state<Record<string, boolean>>({})
	$effect(() => {
		blocksClose =
			editorOpen ||
			toDelete !== undefined ||
			pendingImport !== undefined ||
			Object.values(rowMenuOpen).some(Boolean)
	})

	// Parked with the section, but only once there is nothing to lose: an editor left
	// open behind another section would go on reporting `blocksClose` and the modal
	// would refuse to close with nothing on screen explaining why. An editor holding
	// unsaved text is the opposite case — it stays open so it keeps blocking, and the
	// modal's Escape brings the user back to it rather than discarding the body.
	$effect(() => {
		if (!active && !contentChanged && !pathChanged) editorOpen = false
	})

	/** The list takes focus when this section is the one on screen.
	 *
	 * Without it the keyboard here is unreachable by the way people arrive: the click
	 * on "Skills" in the settings sidebar leaves that button focused, and the sidebar
	 * answers Up/Down itself (`arrowTabNav`), so the arrows would walk the sections
	 * rather than the skills. Taking focus is also what makes the keys unambiguous —
	 * a control keeps Space and Enter, and none holds them once the list has focus. */
	let listEl: HTMLDivElement | undefined = $state(undefined)
	$effect(() => {
		if (!active || editorOpen) return
		const el = listEl
		untrack(() => {
			if (el && !el.contains(document.activeElement)) el.focus({ preventScroll: true })
		})
	})

	/** Escape leaves the editor rather than the whole modal: `blocksClose` stops the
	 * modal's own handler, so this is the only thing left to answer the key. */
	function onKeydown(event: KeyboardEvent) {
		// Every section stays mounted while the modal is open, and `stopPropagation`
		// does nothing between listeners on `window`: without this, a key aimed at the
		// section on screen is answered by the four behind it too.
		if (!active) return
		// A dialog of ours is up and owns the keyboard.
		if (toDelete !== undefined || pendingImport !== undefined) return
		if (editorOpen) {
			if (event.key !== 'Escape') return
			event.preventDefault()
			event.stopPropagation()
			closeEditor()
			return
		}
		listKeydown(event)
	}

	/** DOM id of a row, so the highlight can bring itself into view. */
	function entryDomId(key: string): string {
		return `wm-skill-entry-${key}`
	}

	/** Moving the mouse over the list hands it back to hover, highlight and all.
	 * Dropping the key as well as the drawing is the point: a highlight that is no
	 * longer lit is still a target Space would act on, and the row under the pointer
	 * is the one the user is looking at.
	 *
	 * Driven by a real movement rather than by `mouseenter`, which the browser also
	 * fires when rows arrive under a stationary pointer — every scroll the keyboard
	 * itself causes, and every collapse — and would hand control back to a mouse
	 * nobody touched, restarting the walk at the top of the list. */
	function releaseKeyboardOnMove(event: MouseEvent) {
		if (!keyboardActive) return
		const over = event.target as HTMLElement | null
		if (!over?.closest?.('[id^="wm-skill-entry-"]')) return
		keyboardActive = false
		highlightedKey = undefined
	}

	function setHighlight(key: string | undefined) {
		highlightedKey = key
		keyboardActive = true
		// Taking the highlight means taking the keyboard: a control left focused would
		// keep Space and Enter (the rule in `listKeydown`) and act while a different row
		// is the lit one. Focus is left behind by ordinary use — a row's switch holds it
		// after a plain click, the settings sidebar after the click that opened this
		// section — so this is the common path, not a corner.
		// The list itself is the exception: it holds focus so these keys have somewhere
		// to belong, and blurring it would drop the modal out of the tab order.
		const focused = document.activeElement
		if (focused instanceof HTMLElement && focused !== document.body && focused !== listEl) {
			focused.blur()
			listEl?.focus({ preventScroll: true })
		}
		if (key === undefined) return
		// After the collapse or the list has re-rendered, or the row scrolled to is the
		// one that was on screen before the key.
		tick().then(() =>
			document.getElementById(entryDomId(key))?.scrollIntoView({ block: 'nearest' })
		)
	}

	/** Wraps at both ends, like the chat's other keyboard lists. An unhighlighted list
	 * starts at the row the direction points at. */
	function moveHighlight(step: number) {
		// A row reached with Tab is where the walk carries on from: the focused row is
		// the one the user is on, whether or not the keyboard drew a highlight there.
		const focusedRow = (document.activeElement as HTMLElement | null)?.closest?.(
			'[id^="wm-skill-entry-"]'
		)
		const from = focusedRow ? focusedRow.id.replace('wm-skill-entry-', '') : highlightedKey
		const at = entries.findIndex((e) => e.key === from)
		if (at < 0) {
			setHighlight(entries[step > 0 ? 0 : entries.length - 1].key)
			return
		}
		setHighlight(entries[(at + step + entries.length) % entries.length].key)
	}

	/** Up and Down walk the list, Left and Right fold a folder, Space flips the switch
	 * under the highlight and Enter opens the skill.
	 *
	 * Keys this leaves alone keep their meaning: Left and Right with a skill (or
	 * nothing) highlighted still step between this list and the editor, which is
	 * `PagedContent` reading the same event once we have not answered it. */
	function listKeydown(event: KeyboardEvent) {
		if (entries.length === 0) return
		if (event.metaKey || event.ctrlKey || event.altKey || event.defaultPrevented) return
		// What matters is whether the key is meant for something else, not where focus
		// happens to sit — clicking this section in the settings sidebar leaves that
		// button focused, and a subtree test would answer the first key and refuse every
		// one after it. Only a control that reads these keys itself keeps them.
		// Text entry only. Every `Toggle` in the list hides a real checkbox behind its
		// label (frontend/AGENTS.md), and that checkbox holds focus after a plain click,
		// so refusing all `input` would kill the arrows on the most ordinary interaction
		// there is here.
		const focused = document.activeElement as HTMLElement | null
		const typing =
			'input:not([type="checkbox"]):not([type="radio"]), textarea, select, [contenteditable="true"], [role="listbox"]'
		if (focused?.closest?.(typing)) return
		// Space and Enter belong to a focused control — a row's switch, a folder header,
		// the buttons above the list — since that is where the user is. The list's own
		// container is not one of those: holding focus there is what lets these keys
		// reach the lit row. The arrows below take focus off any control, so once they
		// have moved the highlight these act on it.
		const activating = event.key === ' ' || event.key === 'Enter'
		const control = focused?.closest?.('button, a, input, select, textarea, [role="button"]')
		if (activating && (control || !keyboardActive)) return
		const current = entries.find((e) => e.key === highlightedKey)
		const answer = () => {
			event.preventDefault()
			event.stopPropagation()
		}
		if (event.key === 'ArrowDown') {
			answer()
			moveHighlight(1)
		} else if (event.key === 'ArrowUp') {
			answer()
			moveHighlight(-1)
		} else if (event.key === 'ArrowRight' && current?.kind === 'folder') {
			answer()
			// Open it, or step into what opening it revealed — the next row down is this
			// folder's first child.
			if (collapsed[current.node.path]) collapsed[current.node.path] = false
			else moveHighlight(1)
		} else if (event.key === 'ArrowRight' && current?.kind === 'skill') {
			// Forward from a lit skill is that skill. Left unanswered the key reaches
			// `PagedContent`, which steps to the editor page — showing whichever skill it
			// was last parked on, not this one.
			answer()
			openSkill(current.skill)
		} else if (event.key === 'ArrowLeft' && current !== undefined) {
			if (current.kind === 'folder' && !collapsed[current.node.path]) {
				answer()
				collapsed[current.node.path] = true
			} else if (current.parentKey !== undefined) {
				answer()
				setHighlight(current.parentKey)
			}
		} else if (event.key === 'Enter' && current !== undefined) {
			answer()
			if (current.kind === 'skill') openSkill(current.skill)
			else collapsed[current.node.path] = !collapsed[current.node.path]
		} else if (event.key === ' ' && current !== undefined) {
			answer()
			if (current.kind === 'skill') void toggle(current.skill.path, !current.skill.enabled)
			else void toggleFolder(current.node, !folderEnabled(current.node))
		}
	}

	function closeEditor() {
		editorOpen = false
	}

	/** Left and Right step between the two pages, which is `PagedContent` answering the
	 * arrows once it is given this. Forward only goes somewhere once the editor has been
	 * opened: before that it holds no skill. `editing` is deliberately not cleared on the
	 * way out — both entry points set it, so it stays the skill the parked page shows. */
	function navigate(key: string) {
		if (key === 'list') closeEditor()
		else if (editorSeq > 0) editorOpen = true
	}

	// Rows describe one workspace. A switch while the section is open must not leave
	// A's rows on screen while the actions below target B: same path, different
	// skill, and delete would remove the wrong one.
	let loadSeq = 0
	$effect(() => {
		const target = ws
		untrack(() => {
			loadSeq++
			skills = []
			// Keyed by path, and another workspace's folders and skills are not these.
			collapsed = {}
			highlightedKey = undefined
			listNotice = undefined
			toDelete = undefined
			pendingImport = undefined
			// The editor holds one workspace's skill but saves to whatever `ws` is by
			// then, so a switch mid-edit would write A's body into B at the same path.
			// Closing it is the honest outcome: there is no version of that save the
			// user asked for. This one closes even on unsaved text, unlike parking the
			// section, because the text can no longer be saved where it came from.
			editorOpen = false
			editing = undefined
			void loadSkills(target)
		})
	})

	async function loadSkills(target = ws) {
		refreshForkPending()
		// Checked before the sequence is taken, not only after the await: a stale
		// action calling refresh(A) would otherwise claim the newest sequence and make
		// the legitimate load for B discard its own result, leaving the section on an
		// empty state for a workspace that has skills.
		if (!target || target !== ws) return
		const seq = ++loadSeq
		loading = true
		loadError = undefined
		try {
			const { skills: found, truncated } = await listSkillResources(target, $userStore ?? undefined)
			// Newest-request-wins is not enough on its own: an action started in A and
			// finishing after a switch to B holds the newest sequence, and would put
			// A's rows on screen under B — where the next row action would edit or
			// delete that path in B. The workspace has to still be the one asked for.
			if (seq !== loadSeq || target !== ws) return
			skills = found.map((s) => ({ ...s, enabled: isSkillEnabled(target, s.path) }))
			// A notice, not `loadError`: that one replaces the list, and a truncated
			// read still has skills worth showing.
			listNotice = truncated
				? `Showing the first ${found.length} skills; this workspace has more. Delete unused ones so the rest are listed here.`
				: undefined
			// Seeded rather than filled by the bindings: an unset entry would hand
			// DropdownV2 an `undefined` open state instead of a closed one.
			rowMenuOpen = Object.fromEntries(found.map((s) => [s.path, false]))
		} catch (e) {
			if (seq !== loadSeq || target !== ws) return
			// Without this the section would render the empty state, which reads as
			// "this workspace has no skills" rather than "we could not load them".
			loadError = e.body ?? e.message
		} finally {
			if (seq === loadSeq) loading = false
		}
	}

	async function toggle(p: string, enabled: boolean) {
		if (blockedByPendingFork()) return
		if (!setSkillEnabled(ws, p, enabled)) {
			sendUserToast('Could not save this choice for this account.', true)
			return
		}
		const skill = skills.find((s) => s.path === p)
		if (skill) skill.enabled = enabled
		// Whether people turn skills off at all. Never the skill itself: a path is
		// workspace-authored text.
		logFeatureUsage('ai_session', 'skill_toggle', { key: enabled ? 'on' : 'off', workspace: ws })
		// The prompt lists exactly the skills that are on, so it has to be rebuilt
		// before the next message rather than on the next mode change.
		await aiChatManager.refreshGlobalSkills(ws)
	}

	/** A folder is on only when everything under it is, so its switch always offers
	 * the completing action: a folder with one skill off turns fully on first. */
	function folderEnabled(node: SkillTreeNode<Row>): boolean {
		return nodeSkills(node).every((s) => s.enabled)
	}

	async function toggleFolder(node: SkillTreeNode<Row>, enabled: boolean) {
		if (blockedByPendingFork()) return
		for (const skill of nodeSkills(node)) {
			if (skill.enabled === enabled) continue
			if (!setSkillEnabled(ws, skill.path, enabled)) {
				sendUserToast('Could not save this choice for this account.', true)
				return
			}
			skill.enabled = enabled
		}
		// Once for the folder, not once per skill: what this counts is the action a
		// person took. Never the folder itself, which is workspace-authored text.
		logFeatureUsage('ai_session', 'skill_toggle', { key: enabled ? 'on' : 'off', workspace: ws })
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

	/** Both the state the form reads and what Monaco shows. `SimpleEditor` takes `code`
	 * when it builds its model and never again — and this one outlives a visit, since
	 * the editor page stays mounted — so an assignment alone would leave the body on
	 * screen from the last skill opened. */
	function setContent(next: string) {
		content = next
		bodyEditor?.setCode(next)
	}

	function openCreate() {
		if (blockedByPendingFork()) return
		editorSeq++
		editing = undefined
		setContent(SKILL_TEMPLATE)
		originalContent = SKILL_TEMPLATE
		path = ''
		pathDirty = false
		detailMode = 'edit'
		editorOpen = true
	}

	/** Back to what the editor opened with: the sample for a new skill, the saved
	 * body for one being edited. */
	function resetContent() {
		setContent(originalContent)
	}

	/** Always lands on the rendered skill: reading one is the common reason to open it,
	 * and the header's View/Edit switch is one click from the other half. */
	async function openSkill(skill: Row) {
		// Pinned across the await, and re-checked after it: closing the editor on a
		// workspace switch would not stop this from reopening it with the old
		// workspace's content, which `submitEditor` would then save into the new one.
		const source = ws
		try {
			const instructions = await readSkillBody(source, skill.path)
			if (source !== ws) return
			setContent(
				buildSkillMd({
					name: skill.name,
					description: skill.description,
					instructions
				})
			)
			originalContent = content
			editorSeq++
			editing = skill
			path = skill.path
			pathDirty = false
			detailMode = 'view'
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
		if (blockedByPendingFork()) return
		if (!('skill' in validated) || !path || pathError) return
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
				// A move leaves the old path's state behind on a path that no longer
				// exists; carry it over so an edit that renames does not switch the
				// skill back on, and does not inherit whatever the new path held.
				if (editing.path !== path) {
					const wasEnabled = isSkillEnabled(target, editing.path)
					forgetSkill(target, editing.path)
					setSkillEnabled(target, path, wasEnabled)
				}
			} else {
				await saveSkillResource(
					target,
					path,
					validated.skill.description,
					validated.skill.instructions
				)
				// A skill written here now is not the one someone turned off at this path
				// earlier, so it starts from the default rather than inheriting that.
				forgetSkill(target, path)
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
			// A later skill at this path is a different one, and must not inherit a
			// decision aimed at this one.
			forgetSkill(target, skill.path)
			sendUserToast(`Deleted ${skill.path}`)
			await refresh(target)
		} catch (e) {
			sendUserToast(`Failed to delete skill: ${e.body ?? e.message}`, true)
		}
	}

	async function refresh(target = ws) {
		await loadSkills(target)
		// `refreshGlobalSkills` blanks `globalSkills` when the workspace it is handed is
		// not the one the chat is on, so a refresh landing after a switch would take B's
		// skills out of the system prompt entirely.
		if (target !== ws) return
		await aiChatManager.refreshGlobalSkills(target)
	}

	/** Where the file sat in the chosen folder, as the directory picker reports it. */
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
					forgetSkill(target, dest)
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

<svelte:window onkeydown={onKeydown} onmousemove={releaseKeyboardOnMove} />

<!-- The list and the editor are levels of one panel, so moving between them slides
     rather than cuts. Warmed once this panel is the one on screen: the editor page
     carries a Monaco instance, and building it on the first click lands that work
     inside the transition, which reads as the animation being broken rather than as a
     cost. Warmed here rather than at mount so a modal opened for its other sections
     never pays for it. -->
<PagedContent
	warm={active}
	class="grow min-h-0"
	current={editorOpen ? 'editor' : 'list'}
	onNavigate={active ? navigate : undefined}
	pages={[
		{ key: 'list', content: listPage },
		{ key: 'editor', content: editorPage }
	]}
/>

{#snippet listPage()}
	<!-- Stable gutter: collapsing a folder can take the list under the scrollable
	     height, and without the reserved space every row would jump sideways as the
	     scrollbar came and went. Focusable so the keys have somewhere to belong — see
	     the effect that focuses it. -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		bind:this={listEl}
		tabindex="-1"
		class="grow min-h-0 overflow-y-auto pr-2 outline-none"
		style="scrollbar-gutter: stable;"
	>
		<Section
			label="Skills"
			description="Reusable instruction sets the assistant loads when they apply. Every skill in the workspace is on; turning one off is personal to you and to this workspace."
		>
			{#snippet action()}
				<div class="flex items-center gap-2 shrink-0">
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: FolderUp }}
						disabled={saving || forkPending}
						onClick={() => folderInput?.openPicker()}
					>
						Import a folder
					</Button>
					<Button
						variant="accent"
						unifiedSize="sm"
						startIcon={{ icon: Plus }}
						disabled={saving || forkPending}
						onClick={openCreate}
					>
						New skill
					</Button>
				</div>
			{/snippet}

			<!-- The picker itself. Hidden because the affordance is the button above, but
			     still `FileInput`: it owns reading and filtering the folder, and a raw input
			     here would be a second implementation of that to keep in step. -->
			<div class="hidden">
				<FileInput
					bind:this={folderInput}
					folderOnly
					bind:files={importFiles}
					on:change={onDirSelected}
				/>
			</div>

			{#if forkPending}
				<Alert type="info" title="This session has no workspace yet" size="xs" class="mb-4">
					Skills are read-only until the first message creates this session's fork. Editing one or
					turning it off now would apply to the parent workspace and stop applying once the fork is
					created.
				</Alert>
			{/if}
			{#if listNotice}
				<Alert type="warning" title="Not all skills are shown" size="xs" class="mb-4">
					{listNotice}
				</Alert>
			{/if}
			{#if loading}
				<div class="text-xs text-secondary p-4 text-center">Loading skills…</div>
			{:else if loadError}
				<div class="text-xs text-red-600 dark:text-red-400">
					Failed to load skills: {loadError}
				</div>
			{:else if skills.length === 0}
				<EmptyState
					icon={BookOpen}
					title="No skills yet"
					description="A skill is a SKILL.md the assistant reads when it applies. Write one here, or import a folder of them."
					action={{
						label: 'New skill',
						icon: Plus,
						onClick: openCreate,
						disabled: saving || forkPending
					}}
				/>
			{:else}
				<div class="flex flex-col gap-0.5">
					{#if grouped}
						{#each tree as node (node.path)}
							{@render folder(node, true)}
						{/each}
					{:else}
						{#each skills as skill (skill.path)}
							{@render row(skill)}
						{/each}
					{/if}
				</div>
			{/if}
		</Section>
	</div>
{/snippet}

<!-- One skill. A snippet rather than markup inside the list, because the tree
     renders the same row at every depth. -->
{#snippet row(skill: Row)}
	{#snippet icon()}
		<BookOpen size={16} class="text-tertiary" />
	{/snippet}
	{#snippet title()}
		<span class="truncate leading-5">
			<!-- Grouped, the folder above the row is what tells two skills of the same
			     name apart; flat, the path has to do it. -->
			{!grouped && ambiguous.has(skill.name) ? skill.path : skill.name}
		</span>
	{/snippet}
	{#snippet subtitle()}{skill.description}{/snippet}
	{#snippet trailing()}
		<Toggle
			size="xs"
			disabled={forkPending}
			checked={skill.enabled}
			on:change={async (e) => await toggle(skill.path, e.detail)}
		/>
		<DropdownV2
			size="sm"
			bind:open={rowMenuOpen[skill.path]}
			items={[
				{
					// One entry for both halves of the detail: it opens on the rendered
					// skill, and editing is the switch in its header.
					displayName: 'Manage skill',
					icon: skill.canWrite ? Pencil : Eye,
					action: () => openSkill(skill)
				},
				{
					displayName: 'Delete',
					icon: Trash2,
					type: 'delete',
					disabled: !skill.canWrite || forkPending,
					action: () => (toDelete = skill)
				}
			]}
		/>
	{/snippet}
	<ListRow
		{icon}
		{title}
		{trailing}
		id={entryDomId(skillKey(skill.path))}
		highlighted={keyboardActive ? highlightedKey === skillKey(skill.path) : undefined}
		subtitle={skill.description ? subtitle : undefined}
		onClick={() => openSkill(skill)}
	/>
{/snippet}

<!-- One folder and everything under it. Recursive: a path may name folders below
     the `u/x` or `f/x` root it starts from. -->
{#snippet folder(node: SkillTreeNode<Row>, root: boolean)}
	<div>
		<!-- The switch sits beside the header button rather than inside it: a switch
		     nested in a button would fold the folder on every flip. `pr-14` clears the
		     row menu column, so a folder's switch stands in the same column as the
		     switches of the rows under it. -->
		<div
			id={entryDomId(folderKey(node.path))}
			class="w-full flex items-center gap-2 pr-14 rounded-md {keyboardActive
				? highlightedKey === folderKey(node.path)
					? 'bg-surface-hover'
					: ''
				: 'hover:bg-surface-hover'}"
		>
			<button
				type="button"
				class="grow min-w-0 flex items-center gap-2 px-2 py-2 text-left"
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
				<!-- Explicitly normal: the header is a button, and buttons carry a heavier
				     weight of their own that the label would otherwise inherit. -->
				<span class="text-xs font-normal text-emphasis truncate">{node.label}</span>
				<span class="text-2xs text-secondary">{countSkills(node)}</span>
			</button>
			<Toggle
				size="xs"
				disabled={forkPending}
				checked={folderEnabled(node)}
				on:change={async (e) => await toggleFolder(node, e.detail)}
			/>
		</div>
		{#if !collapsed[node.path]}
			<div class="pl-4 flex flex-col gap-0.5">
				{#each node.children as child (child.path)}
					{@render folder(child, false)}
				{/each}
				{#each node.skills as skill (skill.path)}
					{@render row(skill)}
				{/each}
			</div>
		{/if}
	</div>
{/snippet}

{#snippet editorPage()}
	<!-- The editor takes the panel over rather than opening on top of it: a form
	     stacked on the settings modal leaves two surfaces arguing over which one a
	     click or an Escape belongs to. -->
	<!-- Same reserved gutter as the list, so the slide between the two pages does not
	     shift what is under the cursor. -->
	<div class="grow min-h-0 overflow-y-auto pr-2" style="scrollbar-gutter: stable;">
		<!-- Sticky so the way back is always one click away, however far the page scrolls. -->
		<div class="flex sticky top-0 z-10 bg-surface pb-1">
			<Button
				variant="subtle"
				unifiedSize="xs"
				startIcon={{ icon: ArrowLeft }}
				btnClasses="text-secondary"
				onClick={closeEditor}
			>
				Skills
			</Button>
		</div>
		<!-- `headerClass` keeps a long skill name on one line: the View/Edit group shares
	     the header row and would otherwise wrap the title under itself. -->
		<Section
			label={editing ? editing.name : 'New skill'}
			wrapperClass="mt-1"
			headerClass="min-w-0 truncate pr-2"
		>
			{#snippet action()}
				{#if editing}
					<!-- The group sizes itself to the header row otherwise, leaving the title
				     no room; this keeps it to its content and hard against the right. -->
					<div class="flex justify-end shrink-0">
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
					</div>
				{/if}
			{/snippet}
			<div class="w-full flex flex-col gap-3">
				<!-- Both halves stay mounted and only their display swaps: an `{#if}` here would
				     dispose Monaco every time a reader glanced at the rendered skill and rebuild
				     it on the way back — the work `warm` exists to have done already. -->
				<div class="{detailMode === 'view' ? 'flex' : 'hidden'} flex-col gap-3">
					{#if parsed.description}
						<p class="text-xs text-secondary">{parsed.description}</p>
					{/if}
					<div class="border rounded-md p-3 overflow-auto space-y-2 {markdownProse.sm}">
						<Markdown md={parsed.instructions} plugins={[gfmPlugin()]} />
					</div>
				</div>
				<div class="{detailMode === 'edit' ? 'flex' : 'hidden'} flex-col gap-3">
					<Label label="Path">
						<span class="text-xs font-normal text-secondary">
							The path decides who can use this skill. Under <span class="font-mono"
								>u/{$userStore?.username ?? 'you'}</span
							> it is yours alone; in a folder, everyone who can read that folder can turn it on for
							their own chats.
						</span>
						{#key editorSeq}
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
						{/key}
					</Label>
					<!-- The same editor `/resources` gives these resources, so a skill reads the
				     same in both places. `fixedOverflowWidgets` off keeps its autocomplete
				     inside the panel instead of clipped behind it. -->
					<div class="border border-border-light rounded-md overflow-hidden">
						<SimpleEditor
							bind:this={bodyEditor}
							autoHeight
							lang="markdown"
							bind:code={content}
							fixedOverflowWidgets={false}
							class="min-h-24"
						/>
					</div>
					<div class="flex items-center justify-between gap-2">
						<span class="text-2xs text-red-500 min-w-0">{contentError ?? ''}</span>
						<div class="flex items-center gap-2">
							<Button
								onClick={resetContent}
								variant="subtle"
								unifiedSize="sm"
								startIcon={{ icon: RotateCcw }}
								disabled={saving || !contentChanged}
								title={editing
									? 'Discard unsaved changes to this skill'
									: 'Restore the sample skill'}
							>
								Reset
							</Button>
							<Button
								onClick={submitEditor}
								variant="accent"
								unifiedSize="sm"
								startIcon={{ icon: editing ? Pencil : Plus }}
								disabled={!canSave}
							>
								{editing ? 'Save skill' : 'Add skill'}
							</Button>
						</div>
					</div>
				</div>
			</div>
		</Section>
	</div>
{/snippet}

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
		This deletes the resource at <span class="font-semibold">{toDelete?.path}</span>, so everyone
		who can read it loses the skill.
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
			Skills are added under <span class="font-mono">{defaultOwner()}</span>. Move one to a shared
			folder from the resources page to share it.
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
