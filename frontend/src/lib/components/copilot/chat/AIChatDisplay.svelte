<script lang="ts">
	import AIChatMessage from './AIChatMessage.svelte'
	import AppAvailableContextList from './AppAvailableContextList.svelte'
	import ChatContextPicker from './ChatContextPicker.svelte'
	import { type Snippet } from 'svelte'
	import {
		AlertTriangle,
		ArrowDown,
		AtSign,
		BookOpen,
		ChevronDown,
		ChevronsRight,
		CheckIcon,
		ClipboardList,
		FileText,
		Folder,
		Hand,
		HistoryIcon,
		KeyRound,
		MousePointer2,
		Plug,
		Plus,
		TextSelect,
		X,
		XIcon
	} from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { fade } from 'svelte/transition'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { pendingUserAction, pendingUserActionDetail, type DisplayMessage } from './shared'
	import { PLAN_MODE_TEXT_COLOR, PLAN_MODE_TRIGGER_CLASS } from './planMode'
	import { PLAN_MODE_MESSAGES } from './planModeMessages'
	import type { ContextElement } from './context'
	import ChatQuickActions from './ChatQuickActions.svelte'
	import ContextUsageIndicator from './ContextUsageIndicator.svelte'
	import AIChatModelSettings from './AIChatModelSettings.svelte'
	import AssistantSettingsModal from './AssistantSettingsModal.svelte'
	import { SkillsMenu } from './skills/skillsMenu.svelte'
	import { McpMenu } from '$lib/components/mcp/mcpMenu.svelte'
	import ChatMode from './ChatMode.svelte'
	import DatatableCreationPolicy from './DatatableCreationPolicy.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { twMerge } from 'tailwind-merge'
	import { AIAutonomyMode, AIMode } from './AIChatManager.svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import ChatTypingIndicator from './ChatTypingIndicator.svelte'
	import AIChatInput from './AIChatInput.svelte'
	import AttachedFilesBar from './files/AttachedFilesBar.svelte'
	import QueuedMessageChip from './QueuedMessageChip.svelte'
	import JobsSegment from './JobsSegment.svelte'
	import { getModifierKey } from '$lib/utils'
	import type { SelectedContext } from './app/core'
	import { type FileToAttach } from './files/attachedFiles.svelte'
	import { isImageFile } from './imageUtils'
	import {
		hasFileSystemAccess,
		pickDirectory,
		handlesFromDataTransfer,
		isDirectoryHandle,
		isFileHandle,
		readDroppedEntries
	} from './files/fsAccess'
	import { sendUserToast } from '$lib/toast'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import { base } from '$lib/base'

	const MAX_YOLO_TOOLTIP_TOOLS = 8
	const aiChatManager = getAiChatManager()

	// The user spent their one-time free Windmill AI grant: there is no model left to send
	// to, so say so in the thread itself rather than only failing on send.
	let freeTierExhausted = $derived($copilotInfo.freeTier?.exhausted === true)
	// Still on the free grant: keep how much is left in view right above the composer, so
	// running out isn't a surprise. Once spent, the exhausted banner replaces it.
	let freeTier = $derived($copilotInfo.freeTier)
	let freeTierUsedPct = $derived(Math.min(100, Math.round((freeTier?.used_ratio ?? 0) * 100)))
	let showFreeTierUsage = $derived(!!freeTier && !freeTier.exhausted)

	// One row per autonomy posture, in picker order, so adding one touches only this
	// table. `isAvailable` hides the postures that would do nothing in the current AI
	// mode, which is why the picker can be shorter than this list.
	type AutonomyAvailability = {
		autoAcceptEditsAvailable: boolean
		autoAcceptToolConfirmationsAvailable: boolean
		planModeAvailable: boolean
	}
	type AutonomyModeOption = {
		mode: AIAutonomyMode
		label: string
		shortLabel?: string
		icon: typeof Hand
		iconColor: string
		/** Tints the whole trigger, not just its icon. Only plan mode needs it. */
		triggerClass?: string
		tooltip: (a: AutonomyAvailability) => string
		isAvailable: (a: AutonomyAvailability) => boolean
	}
	// The one posture available everywhere, so also the fallback for a mode the
	// current AI mode does not offer.
	const askPermissionOption: AutonomyModeOption = {
		mode: AIAutonomyMode.DEFAULT,
		label: 'Ask permission',
		icon: Hand,
		iconColor: 'text-secondary',
		tooltip: (a) =>
			a.autoAcceptEditsAvailable
				? 'Requires confirmation for edits and tool calls.'
				: 'Requires confirmation for tool calls.',
		isAvailable: () => true
	}
	const autonomyModeOptions: AutonomyModeOption[] = [
		{
			mode: AIAutonomyMode.PLAN,
			label: 'Plan (read-only)',
			shortLabel: 'Plan',
			icon: ClipboardList,
			iconColor: PLAN_MODE_TEXT_COLOR,
			triggerClass: PLAN_MODE_TRIGGER_CLASS,
			tooltip: () =>
				'Read-only: the assistant researches and drafts a plan for your approval before it can change anything.',
			isAvailable: (a) => a.planModeAvailable
		},
		askPermissionOption,
		{
			mode: AIAutonomyMode.ACCEPT_EDIT,
			label: 'Auto-accept edits',
			icon: ChevronsRight,
			iconColor: 'text-accent',
			tooltip: () =>
				'Automatically accepts script and flow edits. Tool calls still ask for confirmation.',
			isAvailable: (a) => a.autoAcceptEditsAvailable
		},
		{
			mode: AIAutonomyMode.YOLO,
			label: 'Yolo (bypass permissions)',
			shortLabel: 'Yolo',
			icon: ChevronsRight,
			iconColor: 'text-red-500',
			tooltip: (a) =>
				a.autoAcceptEditsAvailable
					? 'Automatically accepts script and flow edits plus tool confirmations.'
					: 'Automatically accepts tool confirmations.',
			isAvailable: (a) => a.autoAcceptToolConfirmationsAvailable
		}
	]
	const autonomyModeOption = (mode: AIAutonomyMode) =>
		autonomyModeOptions.find((o) => o.mode === mode) ?? askPermissionOption
	const autonomyModeLabel = (mode: AIAutonomyMode) => {
		const option = autonomyModeOption(mode)
		return option.shortLabel ?? option.label
	}

	let {
		messages,
		pastChats,
		hasDiff,
		diffMode = false, // todo: remove default
		selectedContext = $bindable([]), // todo: remove default
		availableContext = [], // todo: remove default
		loadPastChat,
		deletePastChat,
		saveAndClear,
		askAi = () => {}, // todo: remove default,
		headerLeft,
		headerRight,
		disabled = false,
		disabledMessage = '',
		suggestions = [],
		hideHeader = false,
		hideModeSelector = false,
		wideLayout = false,
		emptyHint,
		inputPreface,
		initialInstructions = undefined,
		onDraftChange = undefined
	}: {
		messages: DisplayMessage[]
		pastChats: { id: string; title: string }[]
		hasDiff?: boolean
		diffMode: boolean
		selectedContext: ContextElement[]
		availableContext: ContextElement[]
		loadPastChat: (id: string) => void
		deletePastChat: (id: string) => void
		saveAndClear: () => void
		askAi?: (instructions: string, options?: { withCode?: boolean; withDiff?: boolean }) => void
		headerLeft?: Snippet
		headerRight?: Snippet
		disabled?: boolean
		disabledMessage?: string
		suggestions?: string[]
		hideHeader?: boolean
		hideModeSelector?: boolean
		// Center the messages + input columns inside a max-w-3xl px-8
		// inner container. The session pane uses this for breathing
		// room; the right-hand global chat panel is narrow enough that
		// the inner padding eats too much horizontal space, so it's
		// off there.
		wideLayout?: boolean
		emptyHint?: Snippet
		inputPreface?: Snippet
		// Seed / observe the main composer's draft text (see AIChatInput).
		initialInstructions?: string
		onDraftChange?: (text: string) => void
	} = $props()

	let aiChatInput: AIChatInput | undefined = $state()
	let assistantSettings: AssistantSettingsModal | undefined = $state()
	// The "+" menu's skill and MCP rows: enough state to check and flip one, with
	// everything else about them behind the assistant settings modal.
	const skillsMenu = new SkillsMenu(aiChatManager, () => assistantSettings?.open('skills'))
	const mcpMenu = new McpMenu(aiChatManager, () => assistantSettings?.open('mcp'))
	let plusMenuOpen = $state(false)
	let editingMessageIndex = $state<number | null>(null)

	// Escape stops the generation when focus is on the chat (or parked on
	// body), but stays with other widgets (e.g. the session's Monaco editor).
	// Capture phase is required: Monaco and mounted-but-closed Modal2
	// instances consume window Escapes before they bubble.
	let panelEl: HTMLDivElement | undefined = $state()
	$effect(() => {
		function onWindowKeydownCapture(e: KeyboardEvent) {
			if (e.key !== 'Escape' || !aiChatManager.loading) return
			const active = document.activeElement
			const focusOnChat =
				!active || active === document.body || (panelEl?.contains(active) ?? false)
			if (!focusOnChat) return
			e.preventDefault()
			// Immediate form: other chat panels' identical listeners must not
			// also cancel on body focus, nor a drawer/modal close on this press.
			e.stopImmediatePropagation()
			aiChatManager.cancel()
		}
		window.addEventListener('keydown', onWindowKeydownCapture, true)
		return () => window.removeEventListener('keydown', onWindowKeydownCapture, true)
	})

	let scrollEl: HTMLDivElement | undefined = $state()
	// Programmatic-scroll guard. `scrollDown()` triggers an async `scroll`
	// event; if a token-append between the scrollTo and the dispatch makes
	// scrollHeight grow, the gap can briefly exceed STICK_TO_BOTTOM_PX and
	// disengage auto-scroll mid-stream. A short cooldown after our own
	// scroll swallows that spurious event without affecting genuine user
	// scrolls (wheel/touch/keyboard are reaction-time orders of magnitude
	// slower than the cooldown).
	const PROGRAMMATIC_SCROLL_COOLDOWN_MS = 120
	let programmaticScrollAt: number | undefined
	// Instant scroll — smooth would animate every token append, racing with
	// the next scrollDown and confusing the onscroll bottom-detection below.
	function scrollDown() {
		if (!scrollEl) return
		programmaticScrollAt = Date.now()
		scrollEl.scrollTo({ top: scrollEl.scrollHeight, behavior: 'auto' })
	}

	let height = $state(0)
	$effect(() => {
		if (aiChatManager.automaticScroll && height) {
			scrollDown()
		}
		// Recompute the scroll-to-latest visibility on every content-height
		// change. `onScroll` only fires for actual scroll events, so without
		// this the arrow can go stale when content grows past the threshold
		// while auto-scroll is disabled (user scrolled up mid-stream).
		if (scrollEl && height) {
			const distance = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight
			showScrollToLatest = distance > SCROLL_TO_LATEST_THRESHOLD_PX
		}
	})

	// Pixel distance from the bottom under which we treat the user as
	// "stuck to the bottom" and re-enable automatic scroll. 8px allows for
	// sub-pixel rounding from scrollTo + the occasional overscroll bounce.
	const STICK_TO_BOTTOM_PX = 8
	// Show the "scroll to latest" arrow only once the user has scrolled
	// meaningfully away from the tail — a couple of message-heights up. Avoids
	// flicker when the auto-scroll lags by a few px during streaming.
	const SCROLL_TO_LATEST_THRESHOLD_PX = 200
	let showScrollToLatest = $state(false)
	function onScroll() {
		if (!scrollEl) return
		const distance = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight
		// Always refresh the arrow visibility — even during the cooldown,
		// because clicking the arrow itself triggers a programmatic scroll
		// whose only event would otherwise be swallowed, leaving the arrow
		// stuck visible after we already reached the bottom.
		showScrollToLatest = distance > SCROLL_TO_LATEST_THRESHOLD_PX
		if (
			programmaticScrollAt !== undefined &&
			Date.now() - programmaticScrollAt < PROGRAMMATIC_SCROLL_COOLDOWN_MS
		) {
			return
		}
		if (distance <= STICK_TO_BOTTOM_PX) {
			aiChatManager.enableAutomaticScroll()
		} else {
			aiChatManager.disableAutomaticScroll()
		}
	}

	function submitSuggestion(suggestion: string) {
		aiChatManager.sendRequest({ instructions: suggestion })
	}

	export function focusInput() {
		aiChatInput?.focusInput()
	}

	$effect(() => {
		if (aiChatInput) {
			aiChatManager.setAiChatInput(aiChatInput)
		}

		return () => {
			aiChatManager.setAiChatInput(null)
		}
	})

	// Also shown for a run held by another tab, labeled with where it is: the
	// dots say a turn is in flight even before the reader reaches the footer
	// note. Remote runs pause nothing and offer no Stop — this tab can't cancel.
	const showTypingIndicator = $derived(aiChatManager.loading || aiChatManager.runHeldElsewhere)

	// The manual `@` context-picker button. Shown in SCRIPT/FLOW (workspace items +
	// code blocks) and APP (datatables, frontend files). Hidden in GLOBAL — there
	// `@`-context is still invoked inline by typing `@` in the input, so the button
	// is redundant. NAVIGATOR/ASK/API don't take @-context at all.
	const showContextPicker = $derived(
		aiChatManager.mode === AIMode.SCRIPT ||
			aiChatManager.mode === AIMode.FLOW ||
			aiChatManager.mode === AIMode.APP
	)

	// File attachment is GLOBAL-mode only.
	const canAttachFiles = $derived(aiChatManager.mode === AIMode.GLOBAL && !disabled)
	// Steers the OS file picker toward text + image formats (soft hint; both attach
	// to the message — text files after a content sniff).
	const TEXT_FILE_ACCEPT =
		'image/*,text/*,.txt,.csv,.tsv,.json,.jsonl,.ndjson,.md,.markdown,.log,.yaml,.yml,.toml,.ini,.cfg,.conf,.env,.xml,.html,.htm,.css,.js,.mjs,.cjs,.ts,.tsx,.jsx,.py,.rb,.rs,.go,.java,.kt,.c,.h,.cpp,.cc,.cs,.php,.sh,.bash,.zsh,.sql,.svelte,.vue,.dockerfile'
	let fileInputEl = $state<HTMLInputElement | null>(null)
	let folderInputEl = $state<HTMLInputElement | null>(null)
	let dragDepth = $state(0)
	const isDraggingFiles = $derived(dragDepth > 0)
	// File System Access API → live re-grantable folder handles (refreshed each turn).
	// Otherwise folders are snapshotted into the browser (via webkitdirectory / dropped-entry
	// walk), same as files. Either way folders display identically.
	const canUseFsAccess = hasFileSystemAccess()

	function reportAddResult(added: string[], rejected: { name: string; reason: string }[]) {
		if (rejected.length === 0) return
		// Single rejected file (e.g. one dropped image): show the precise reason.
		if (added.length === 0 && rejected.length === 1) {
			sendUserToast(`Could not attach "${rejected[0].name}": ${rejected[0].reason}`, true)
			return
		}
		// Otherwise (folders / multi-select): summarize to avoid a flood of toasts. The only
		// per-file rejection left is non-text content (binary files are skipped).
		const lead = added.length
			? `Attached ${added.length}, skipped ${rejected.length}`
			: `Skipped ${rejected.length} file${rejected.length === 1 ? '' : 's'}`
		sendUserToast(`${lead} (non-text).`, added.length === 0)
	}

	async function handleAddFiles(files: FileList | FileToAttach[]) {
		const { added, rejected } = await aiChatManager.attachedFiles.addFiles(files)
		reportAddResult(added, rejected)
	}

	async function addDirHandle(dir: FileSystemDirectoryHandle) {
		const { added, rejected } = await aiChatManager.attachedFiles.addFolder(dir)
		reportAddResult(added, rejected)
	}

	function linkFiles() {
		// Files are always snapshotted (every browser), so the universal picker is fine.
		fileInputEl?.click()
	}

	async function linkFolder() {
		if (!canUseFsAccess) {
			// No File System Access API → pick a folder via the directory input; its files are
			// snapshotted into the browser (no live handle), grouped under the folder name.
			folderInputEl?.click()
			return
		}
		let dir: FileSystemDirectoryHandle | undefined
		try {
			dir = await pickDirectory()
		} catch (e) {
			// The picker threw instead of opening — surface why (e.g. a browser/enterprise
			// policy blocking the File System Access API) rather than appearing to do nothing.
			sendUserToast(
				`Couldn't open the folder picker: ${e instanceof Error ? e.message : String(e)}`,
				true
			)
			return
		}
		if (dir) await addDirHandle(dir)
	}

	function dragHasFiles(e: DragEvent): boolean {
		return Array.from(e.dataTransfer?.types ?? []).includes('Files')
	}

	function onPanelDragEnter(e: DragEvent) {
		if (!canAttachFiles || !dragHasFiles(e)) return
		e.preventDefault()
		dragDepth++
	}
	function onPanelDragOver(e: DragEvent) {
		if (!canAttachFiles || !dragHasFiles(e)) return
		e.preventDefault()
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'
	}
	function onPanelDragLeave(_e: DragEvent) {
		if (!canAttachFiles) return
		dragDepth = Math.max(0, dragDepth - 1)
	}
	async function onPanelDrop(e: DragEvent) {
		dragDepth = 0
		if (!canAttachFiles || !dragHasFiles(e)) return
		e.preventDefault()
		const dt = e.dataTransfer
		if (!dt) return
		// Images and loose text files attach to the message; folders link as session
		// assets. Images are reserved from dt.files BEFORE any await (a send
		// mid-ingestion would land them on the next message), and dt.files is the
		// only place a disk-less drag exists — a cross-tab image resolves every
		// getAsFileSystemHandle() to null.
		const flatFiles = Array.from(dt.files ?? [])
		const topLevelImages = flatFiles.filter(isImageFile)
		const imageWork: Promise<unknown>[] = []
		if (topLevelImages.length > 0) {
			imageWork.push(aiChatInput?.addImages(topLevelImages) ?? Promise.resolve())
		}
		// Text-file routing must await handle/entry resolution before it can call
		// addTextFiles — hold sending across that window (taken BEFORE the first
		// await) or a send mid-resolution would land the drop on the next message.
		const releaseSendHold = aiChatInput?.holdSendForIngestion()
		try {
			await routeDroppedTextAndFolders(dt, flatFiles)
		} finally {
			releaseSendHold?.()
		}
		await Promise.all(imageWork)
	}

	async function routeDroppedTextAndFolders(dt: DataTransfer, flatFiles: File[]) {
		if (canUseFsAccess) {
			// getAsFileSystemHandle calls are kicked off synchronously inside this call.
			const handles = await handlesFromDataTransfer(dt)
			// No handles → nothing beyond dt.files exists; its text files are all there is.
			// Handle-backed files are top-level by definition, so their images are
			// already reserved above — only text files remain to route.
			const looseFiles =
				handles.length === 0
					? flatFiles
					: await Promise.all(handles.filter(isFileHandle).map((h) => h.getFile()))
			// Loose text files attach to the message, like images.
			const textFiles = looseFiles.filter((f) => !isImageFile(f))
			if (textFiles.length > 0) await aiChatInput?.addTextFiles(textFiles)
			// Folders link as a live handle.
			for (const h of handles.filter(isDirectoryHandle)) {
				await addDirHandle(h)
			}
		} else {
			// Fallback (no File System Access API): snapshot dropped files AND folders by walking
			// the legacy webkitGetAsEntry tree. readDroppedEntries reads the entries synchronously
			// (they're only valid during this event) before its first await; if it yields nothing
			// (no entry API), fall back to the flat dt.files.
			const entries = await readDroppedEntries(Array.from(dt.items ?? []))
			const source: FileToAttach[] = entries.length > 0 ? entries : flatFiles
			// Top-level files attach to the message (images were already reserved
			// from dt.files before the walk). Folder children keep riding the
			// session store as a snapshot — including nested images, which are
			// deliberately NOT attached (the FSA path never extracts folder
			// contents either); they are summarized as skipped there.
			const topLevelText: File[] = []
			const folderEntries: FileToAttach[] = []
			for (const entry of source) {
				const file = entry instanceof File ? entry : entry.file
				const nested = !(entry instanceof File) && !!entry.path?.includes('/')
				if (nested) {
					folderEntries.push(entry)
				} else if (!isImageFile(file)) {
					topLevelText.push(file)
				}
			}
			if (folderEntries.length > 0) await handleAddFiles(folderEntries)
			if (topLevelText.length > 0) await aiChatInput?.addTextFiles(topLevelText)
		}
	}

	async function onFileInputChange(e: Event) {
		const input = e.currentTarget as HTMLInputElement
		if (input.files && input.files.length > 0) {
			const picked = Array.from(input.files)
			const imageFiles = picked.filter(isImageFile)
			const textFiles = picked.filter((f) => !isImageFile(f))
			// Reserved before the text work is awaited — see onPanelDrop.
			const imageWork = imageFiles.length > 0 ? aiChatInput?.addImages(imageFiles) : undefined
			if (textFiles.length > 0) await aiChatInput?.addTextFiles(textFiles)
			await imageWork
		}
		input.value = '' // allow re-selecting the same file
	}

	function onFolderInputChange(e: Event) {
		const input = e.currentTarget as HTMLInputElement
		// webkitdirectory files carry webkitRelativePath (`folder/sub/file`); addFiles groups
		// them under the folder and skips junk paths. Snapshot, like a dropped folder.
		if (input.files && input.files.length > 0) void handleAddFiles(input.files)
		input.value = ''
	}
	const autonomyAvailability = $derived({
		autoAcceptEditsAvailable: aiChatManager.autoAcceptEditsAvailable,
		autoAcceptToolConfirmationsAvailable: aiChatManager.autoAcceptToolConfirmationsAvailable,
		planModeAvailable: aiChatManager.planModeAvailable
	})
	const availableAutonomyModeOptions = $derived(
		autonomyModeOptions.filter((option) => option.isAvailable(autonomyAvailability))
	)
	// Fall back to ask-permission when the persisted mode isn't applicable in the
	// current AI mode (e.g. auto-accept edits while in a mode without edits).
	const effectiveAutonomyMode = $derived(
		availableAutonomyModeOptions.some((option) => option.mode === aiChatManager.autonomyMode)
			? aiChatManager.autonomyMode
			: AIAutonomyMode.DEFAULT
	)
	const showAutonomyModeSelector = $derived(!disabled && availableAutonomyModeOptions.length > 1)
	const effectiveAutonomyModeOption = $derived(autonomyModeOption(effectiveAutonomyMode))

	// The typing-dots indicator implies the AI is busy, which is misleading while
	// the loop is parked on the user; surface a text pill instead so users know to
	// act on the tool above.
	const waitingForUserAction = $derived(aiChatManager.loading && !!pendingUserAction(messages))

	// Gated on `loading` because a card restored from history still looks parked:
	// its resolver left with the old page, so the composer must not advertise an
	// answer it cannot deliver.
	const pendingQuestionToolCallId = $derived.by(() => {
		if (!aiChatManager.loading) {
			return undefined
		}
		const pending = pendingUserActionDetail(messages)
		return pending?.action === 'question' ? pending.toolCallId : undefined
	})

	// Get app context for display when in APP mode
	const appContext = $derived.by((): SelectedContext | undefined => {
		if (aiChatManager.mode !== AIMode.APP || !aiChatManager.appAiChatHelpers) {
			return undefined
		}
		return aiChatManager.appAiChatHelpers.getSelectedContext()
	})

	const yoloBypassedTools = $derived.by(() => {
		return aiChatManager.tools
			.filter((tool) => tool.requiresConfirmation === true)
			.map((tool) => ({
				name: tool.def.function.name,
				// confirmationMessage may be a function of the call args, which we don't
				// have here — fall back to the tool name rather than render its source.
				label:
					typeof tool.confirmationMessage === 'string'
						? tool.confirmationMessage
						: tool.def.function.name
			}))
	})
	const visibleYoloBypassedTools = $derived(yoloBypassedTools.slice(0, MAX_YOLO_TOOLTIP_TOOLS))
	const hiddenYoloBypassedToolCount = $derived(
		Math.max(0, yoloBypassedTools.length - visibleYoloBypassedTools.length)
	)
	const showFlowPendingActionControls = $derived(
		(aiChatManager.flowAiChatHelpers?.hasPendingChanges() ?? false) &&
			!aiChatManager.autoAcceptEditsActive
	)
	// A disabled state with no message (a remote hold, a spent free grant) keeps
	// the footer toolbar in place — swapping it for an empty strip would make
	// the model/mode row flash out and back on every remote turn. A state with
	// a real message (archived, AI off) still shows it, hold or not, matching
	// the precedence disabledMessage itself encodes.
	const footerMessageShown = $derived(disabled && disabledMessage !== '')
	const showFooterLeftControls = $derived(
		!footerMessageShown &&
			(showContextPicker ||
				showAutonomyModeSelector ||
				(aiChatManager.mode === AIMode.SCRIPT && hasDiff))
	)
</script>

{#snippet freeTierExhaustedBanner()}
	<div class="my-2">
		<Alert type="info" size="xs" title="Free Windmill AI used up">
			<div class="flex flex-col items-start gap-2">
				<span>
					You have used all of your free Windmill AI tokens. Add your own API key to keep using AI.
				</span>
				<Button
					unifiedSize="2xs"
					variant="accent"
					startIcon={{ icon: KeyRound }}
					href="{base}/workspace_settings?tab=ai"
				>
					Add your own API key
				</Button>
			</div>
		</Alert>
	</div>
{/snippet}

{#snippet freeTierUsageBanner()}
	<div
		class="my-1 flex items-center justify-between gap-2 rounded-md border bg-surface-secondary px-2 py-1"
	>
		<span class="text-xs text-secondary tabular-nums">
			{freeTierUsedPct}% of your free Windmill AI used
		</span>
		<Button
			unifiedSize="2xs"
			variant="default"
			startIcon={{ icon: KeyRound }}
			href="{base}/workspace_settings?tab=ai"
		>
			Configure your API key
		</Button>
	</div>
{/snippet}

<!-- tabindex="-1": clicks on non-focusable chat content must move focus into
the panel, or the Escape-to-stop focus check would wrongly reject them. -->
<div
	class="flex flex-col h-full relative outline-none"
	tabindex="-1"
	bind:this={panelEl}
	ondragenter={onPanelDragEnter}
	ondragover={onPanelDragOver}
	ondragleave={onPanelDragLeave}
	ondrop={onPanelDrop}
	role="region"
	aria-label="AI chat"
>
	{#if isDraggingFiles}
		<div
			class="absolute inset-0 z-50 flex items-center justify-center pointer-events-none rounded-md border-2 border-dashed border-blue-400 bg-blue-500/10"
			transition:fade={{ duration: 100 }}
		>
			<div class="flex flex-col items-center gap-1 text-blue-600 dark:text-blue-300">
				<Plus size={24} />
				<span class="text-sm font-medium">Drop files or images to attach</span>
			</div>
		</div>
	{/if}
	{#if !hideHeader}
		<div
			class="flex flex-row items-center justify-between gap-2 p-2 border-b border-gray-200 dark:border-gray-600"
		>
			<div class="flex flex-row items-center gap-2">
				{@render headerLeft?.()}
				<p class="text-sm font-semibold">Chat</p>
			</div>
			<div class="flex flex-row items-center gap-2">
				<Popover>
					{#snippet trigger()}
						<Button
							on:click={() => {}}
							title="History"
							size="md"
							btnClasses="!p-1"
							startIcon={{ icon: HistoryIcon }}
							iconOnly
							variant="border"
							color="light"
							propagateEvent
						/>
					{/snippet}
					{#snippet content({ close })}
						<div class="p-1 overflow-y-auto max-h-[300px]">
							{#if pastChats.length === 0}
								<div class="text-center text-primary text-xs">No history</div>
							{:else}
								<div class="flex flex-col">
									{#each pastChats as chat (chat.id)}
										<button
											class="text-left flex flex-row items-center gap-2 justify-between hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md p-1 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-transparent dark:disabled:hover:bg-transparent"
											disabled={aiChatManager.loading ||
												aiChatManager.sendInFlight ||
												aiChatManager.runHeldElsewhere}
											title={aiChatManager.runHeldElsewhere
												? 'Wait for the turn in the other tab to switch conversation'
												: aiChatManager.loading || aiChatManager.sendInFlight
													? 'Stop the current answer to switch conversation'
													: undefined}
											onclick={() => {
												loadPastChat(chat.id)
												close()
											}}
										>
											<div
												class="text-xs font-medium w-48 text-ellipsis overflow-hidden whitespace-nowrap flex-1"
												title={chat.title}
											>
												{chat.title}
											</div>
											<Button
												iconOnly
												size="xs2"
												btnClasses="!p-1"
												variant="default"
												startIcon={{ icon: X }}
												on:click={() => {
													deletePastChat(chat.id)
												}}
											/>
										</button>
									{/each}
								</div>
							{/if}
						</div>
					{/snippet}
				</Popover>
				<Button
					title={aiChatManager.runHeldElsewhere
						? 'Wait for the turn in the other tab to start a new chat'
						: 'New chat'}
					disabled={aiChatManager.runHeldElsewhere}
					on:click={() => {
						saveAndClear()
					}}
					size="md"
					btnClasses="!p-1"
					startIcon={{ icon: Plus }}
					iconOnly
					variant="border"
					color="light"
				/>
				{@render headerRight?.()}
			</div>
		</div>
	{/if}
	{#if messages.length === 0}
		{#if emptyHint}
			{@render emptyHint()}
		{:else}
			<span class="text-2xs text-gray-500 dark:text-gray-400 text-center px-2 my-2"
				>You can use {getModifierKey()}L to open or close this chat, and {getModifierKey()}K in the
				script editor to modify selected lines.</span
			>
		{/if}
		{#if freeTierExhausted}
			<div class={wideLayout ? 'w-full max-w-3xl mx-auto px-7' : 'w-full max-w-2xl mx-auto px-3'}>
				{@render freeTierExhaustedBanner()}
			</div>
		{/if}
	{/if}

	{#if messages.length > 0}
		<div class="flex-1 min-h-0 relative">
			<div
				class="absolute inset-0 overflow-y-scroll pt-2 scrollbar-subtle"
				bind:this={scrollEl}
				onscroll={onScroll}
			>
				<div
					class={wideLayout
						? 'w-full max-w-3xl mx-auto px-7 flex flex-col pb-2'
						: 'w-full max-w-2xl mx-auto px-3 flex flex-col pb-2'}
					bind:clientHeight={height}
				>
					{#each messages as message, messageIndex (messageIndex)}
						<AIChatMessage
							{message}
							{messageIndex}
							{availableContext}
							bind:editingMessageIndex
							isLast={messageIndex === messages.length - 1}
						/>
					{/each}
					{#if freeTierExhausted}
						{@render freeTierExhaustedBanner()}
					{/if}
					{#if showTypingIndicator}
						<div
							class={twMerge(
								'sticky z-10 -mt-10 ml-2 self-start pointer-events-none',
								showFlowPendingActionControls ? 'bottom-14' : 'bottom-2'
							)}
						>
							<ChatTypingIndicator
								loading={showTypingIndicator}
								paused={waitingForUserAction}
								label={aiChatManager.runHeldElsewhere
									? 'Running in another tab'
									: aiChatManager.loadingLabel
										? aiChatManager.loadingLabel
										: aiChatManager.compacting
											? 'Compacting conversation'
											: aiChatManager.currentReasoningActive &&
												  !aiChatManager.currentReply &&
												  !aiChatManager.currentReasoning
												? (aiChatManager.reasoningHiddenIndicatorLabel ?? 'Thinking')
												: undefined}
							/>
						</div>
					{/if}
				</div>
			</div>
			{#if showScrollToLatest}
				<div
					transition:fade={{ duration: 120 }}
					class={twMerge(
						'absolute left-1/2 -translate-x-1/2 z-10 rounded-md bg-surface shadow-md',
						showFlowPendingActionControls ? 'bottom-12' : 'bottom-2'
					)}
				>
					<Button
						variant="default"
						unifiedSize="sm"
						iconOnly
						title="Scroll to latest"
						aria-label="Scroll to latest message"
						startIcon={{ icon: ArrowDown }}
						on:click={() => {
							aiChatManager.enableAutomaticScroll()
							scrollDown()
						}}
					/>
				</div>
			{/if}
		</div>
	{/if}

	<div
		class={wideLayout
			? 'relative w-full max-w-3xl mx-auto px-6 pb-2'
			: 'relative w-full max-w-2xl mx-auto px-2 pb-2'}
	>
		{#if showFlowPendingActionControls}
			<div class="absolute -top-10 w-full flex flex-row justify-center gap-2">
				<Button
					startIcon={{ icon: CheckIcon }}
					size="xs"
					variant="default"
					btnClasses="bg-green-500 hover:bg-green-600 text-white hover:text-white"
					onclick={() => {
						aiChatManager.flowAiChatHelpers?.acceptAllModuleActions()
					}}
				>
					Accept all
				</Button>
				<div class="rounded bg-surface">
					<Button
						startIcon={{ icon: XIcon }}
						size="xs"
						variant="default"
						btnClasses="dark:opacity-50 opacity-60 hover:opacity-100"
						onclick={() => {
							aiChatManager.flowAiChatHelpers?.rejectAllModuleActions()
						}}
					>
						Reject all
					</Button>
				</div>
			</div>
		{/if}
		<div>
			<QueuedMessageChip />
			{#if aiChatManager.mode === AIMode.GLOBAL && !aiChatManager.isSessionChat}
				<!-- Standalone Jobs bar for the global side-panel chat. In /sessions the
				     Jobs segment lives inside the session bar (SessionChangesBar). -->
				<div class="mb-1">
					<JobsSegment standalone />
				</div>
			{/if}
			<!-- Message-scoped chips (selected-context / DOM-selector / images) render
			     inside the input box via AIChatInput → ContextTextarea's `leading` snippet;
			     selected context also appears as @mentions in the input (deleting the
			     mention deselects). Hence showContext={false} below. Session-scoped
			     assets (attached files/folders) render in the footer row instead. -->
			{#if inputPreface}
				{@render inputPreface()}
			{/if}
			{#if showFreeTierUsage}
				{@render freeTierUsageBanner()}
			{/if}
			<AIChatInput
				bind:this={aiChatInput}
				bind:selectedContext
				{availableContext}
				{initialInstructions}
				{onDraftChange}
				showContext={aiChatManager.mode !== AIMode.GLOBAL}
				{disabled}
				{pendingQuestionToolCallId}
				isFirstMessage={messages.length === 0}
			/>
			<div
				class="mt-1 flex flex-row flex-wrap items-center gap-x-1.5 gap-y-1"
				class:justify-between={showFooterLeftControls}
				class:justify-end={!showFooterLeftControls}
			>
				{#if showFooterLeftControls}
					<div class="flex flex-row items-center gap-x-1.5 min-w-0 flex-wrap">
						{#if showContextPicker && !disabled}
							<Popover placement="bottom-start">
								{#snippet trigger()}
									<Button
										nonCaptureEvent
										unifiedSize="2xs"
										variant="default"
										title="Add context"
										iconOnly
										startIcon={{ icon: AtSign }}
									/>
								{/snippet}
								{#snippet content({ close })}
									{#if aiChatManager.mode === AIMode.APP}
										<AppAvailableContextList
											{availableContext}
											{selectedContext}
											onSelect={(element) => {
												void aiChatInput?.addContextToSelection(element)
												close()
											}}
										/>
									{:else}
										<ChatContextPicker
											{availableContext}
											{selectedContext}
											onSelect={(element) => {
												void aiChatInput?.addContextToSelection(element)
												aiChatInput?.insertMention(element.title)
												close()
												aiChatInput?.focusInput()
											}}
											onSelectWorkspaceItem={(element) => {
												void aiChatInput?.addContextToSelection(element)
												aiChatInput?.insertMention(element.title)
												close()
												aiChatInput?.focusInput()
											}}
											setShowing={(showing) => {
												if (!showing) close()
											}}
											onSelectFile={(name) => {
												aiChatInput?.insertFileMention(name)
												close()
											}}
										/>
									{/if}
								{/snippet}
							</Popover>
						{/if}
						{#if canAttachFiles}
							<DropdownV2
								items={async () => {
									// Both submenus fetch on the menu's first open, so they start
									// together: awaited inline they queue, and the whole menu —
									// attachments included — waits out two round trips.
									const closeMenu = () => (plusMenuOpen = false)
									const inGlobal = aiChatManager.mode === AIMode.GLOBAL
									const [skillItems, mcpItems] = await Promise.all([
										inGlobal ? skillsMenu.items(closeMenu) : undefined,
										inGlobal ? mcpMenu.items(closeMenu) : undefined
									])
									return [
										{
											displayName: 'Attach file or image',
											icon: FileText,
											action: () => {
												plusMenuOpen = false
												linkFiles()
											}
										},
										{
											// A real (live) link needs the File System Access API; without it the
											// folder is only snapshotted, so call it "Add folder", not "Link folder".
											displayName: canUseFsAccess ? 'Link folder' : 'Add folder',
											icon: Folder,
											tooltip: canUseFsAccess
												? 'Linked live — the assistant reads the folder’s current files from disk and refreshes each turn.'
												: 'Loaded as a snapshot — the folder’s files are copied into your browser (they won’t auto-update). For a live link that refreshes from disk, use a Chromium-based browser (Chrome, Edge).',
											action: () => {
												plusMenuOpen = false
												linkFolder()
											}
										},
										...(skillItems
											? [
													{
														displayName: 'Skills',
														icon: BookOpen,
														separatorTop: true,
														submenuItems: skillItems
													}
												]
											: []),
										...(mcpItems
											? [
													{
														displayName: 'MCP connections',
														icon: Plug,
														separatorTop: !skillItems,
														submenuItems: mcpItems
													}
												]
											: [])
									]
								}}
								placement="bottom-start"
								fixedHeight={false}
								closeOnItemClick={false}
								bind:open={plusMenuOpen}
							>
								{#snippet buttonReplacement()}
									<Tooltip small placement="top">
										<Button
											nonCaptureEvent
											unifiedSize="2xs"
											variant="default"
											iconOnly
											startIcon={{ icon: Plus }}
										/>
										{#snippet text()}
											<div class="max-w-64 text-xs">
												<p class="font-semibold">Attach files or link a folder</p>
												<p class="mt-1">
													Files and images attach to your next message. Images are seen directly;
													file contents stay in your browser and are read on demand.
												</p>
												<p class="mt-1">
													A linked folder is a session-wide resource: the assistant lists, searches,
													and reads its files whenever it needs them.
												</p>
											</div>
										{/snippet}
									</Tooltip>
								{/snippet}
							</DropdownV2>
							<!-- Fallback file picker (used when the File System Access API is unavailable).
							     `accept` only steers the picker; the content sniff at attach is authoritative. -->
							<input
								bind:this={fileInputEl}
								type="file"
								multiple
								accept={TEXT_FILE_ACCEPT}
								class="hidden no-default-style"
								onchange={onFileInputChange}
							/>
							<!-- Fallback folder picker (no File System Access API): webkitdirectory selects a
							     whole folder; its files carry webkitRelativePath and are snapshotted. -->
							<input
								bind:this={folderInputEl}
								type="file"
								multiple
								webkitdirectory
								class="hidden no-default-style"
								onchange={onFolderInputChange}
							/>
						{/if}
						{#if showAutonomyModeSelector}
							<DropdownV2
								items={() =>
									availableAutonomyModeOptions.map((option) => ({
										displayName: option.label,
										selected: effectiveAutonomyMode === option.mode,
										action: () => aiChatManager.setAutonomyMode(option.mode)
									}))}
								placement="bottom-start"
								fixedHeight={false}
								customWidth={240}
							>
								{#snippet buttonReplacement()}
									<Button
										nonCaptureEvent
										unifiedSize="2xs"
										variant="default"
										title={effectiveAutonomyModeOption.tooltip(autonomyAvailability)}
										btnClasses={effectiveAutonomyModeOption.triggerClass ?? ''}
										startIcon={{
											icon: effectiveAutonomyModeOption.icon,
											classes: effectiveAutonomyModeOption.iconColor
										}}
										endIcon={{ icon: ChevronDown }}
									>
										{autonomyModeLabel(effectiveAutonomyMode)}
									</Button>
								{/snippet}
							</DropdownV2>
						{/if}
						{#if effectiveAutonomyMode === AIAutonomyMode.PLAN}
							<span class="text-2xs text-secondary">{PLAN_MODE_MESSAGES.modeNote}</span>
						{/if}
						{#if effectiveAutonomyMode === AIAutonomyMode.YOLO && aiChatManager.autoAcceptToolConfirmationsAvailable}
							<Tooltip small placement="top">
								<AlertTriangle class="w-3 h-3 text-red-500" />
								{#snippet text()}
									<div class="max-w-64 text-xs">
										<p class="font-semibold">
											{aiChatManager.autoAcceptEditsAvailable
												? 'Bypass permissions auto-accepts edits and tool usage.'
												: 'Bypass permissions auto-accepts tool usage.'}
										</p>
										<p class="mt-1">
											{aiChatManager.autoAcceptEditsAvailable
												? 'This can result in edits being applied or tools being called without user confirmation.'
												: 'This can result in tools being called without user confirmation.'}
										</p>
										{#if yoloBypassedTools.length > 0}
											<p class="mt-2 font-semibold">Bypassed in current mode:</p>
											<ul class="mt-1 list-disc pl-4 space-y-0.5">
												{#each visibleYoloBypassedTools as tool (tool.name)}
													<li class="break-words">{tool.label}</li>
												{/each}
											</ul>
											{#if hiddenYoloBypassedToolCount > 0}
												<p class="mt-1">+ {hiddenYoloBypassedToolCount} more</p>
											{/if}
										{:else}
											<p class="mt-2">No tools in the current mode require confirmation.</p>
										{/if}
									</div>
								{/snippet}
							</Tooltip>
						{/if}
						{#if aiChatManager.mode === AIMode.SCRIPT && hasDiff && !disabled}
							<ChatQuickActions {askAi} {diffMode} />
						{/if}
					</div>
				{/if}
				{#if footerMessageShown}
					<div class="text-primary text-xs my-2 px-2">
						<Markdown md={disabledMessage} />
					</div>
				{:else}
					<div class="flex flex-row gap-x-1.5 min-w-0 flex-wrap items-center">
						{#if aiChatManager.mode === AIMode.GLOBAL}
							<AttachedFilesBar />
						{/if}
						{#if !hideModeSelector}
							<ChatMode />
						{/if}
						{#if aiChatManager.mode === AIMode.APP}
							<DatatableCreationPolicy />
						{/if}
						<ContextUsageIndicator />
						<!-- Unconditional: this composer mounts only via `AIChat` ← `SessionWrapper`,
						     and `sessionRuntime` locks a session to GLOBAL, where the settings
						     modal's Instructions section owns the prompt entries. -->
						<AIChatModelSettings promptSettings={false} />
						{#if aiChatManager.mode === AIMode.GLOBAL}
							<AssistantSettingsModal bind:this={assistantSettings} />
						{/if}

						{#if aiChatManager.mode === AIMode.APP && appContext && (appContext.inspectorElement || appContext.codeSelection)}
							{#if appContext.inspectorElement}
								<div
									class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 text-2xs"
									title={appContext.inspectorElement.path}
								>
									<MousePointer2 class="w-3 h-3" />
									<span class="truncate max-w-[60px]">
										{appContext.inspectorElement.tagName.toLowerCase()}{appContext.inspectorElement
											.id
											? `#${appContext.inspectorElement.id}`
											: ''}{appContext.inspectorElement.className
											? `.${appContext.inspectorElement.className.split(' ')[0]}`
											: ''}
									</span>
									<button
										class="hover:bg-purple-200 dark:hover:bg-purple-800/50 rounded p-0.5 -mr-0.5"
										onclick={() => appContext.clearInspector?.()}
										title="Clear element selection"
									>
										<X class="w-2.5 h-2.5" />
									</button>
								</div>
							{/if}
							{#if appContext.codeSelection}
								<div
									class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300 text-2xs"
									title={`${appContext.codeSelection.source}: lines ${appContext.codeSelection.startLine}-${appContext.codeSelection.endLine}`}
								>
									<TextSelect class="w-3 h-3" />
									<span class="truncate max-w-[80px]">
										L{appContext.codeSelection.startLine}-{appContext.codeSelection.endLine}
									</span>
									<button
										class="hover:bg-amber-200 dark:hover:bg-amber-800/50 rounded p-0.5 -mr-0.5"
										onclick={() => appContext.clearCodeSelection?.()}
										title="Clear code selection"
									>
										<X class="w-2.5 h-2.5" />
									</button>
								</div>
							{/if}
						{/if}
					</div>
				{/if}
			</div>
		</div>
		{#if (aiChatManager.mode === AIMode.NAVIGATOR || aiChatManager.mode === AIMode.ASK) && suggestions.length > 0 && messages.filter((m) => m.role === 'user').length === 0 && !disabled}
			<div class="px-2 mt-4">
				<div class="flex flex-col gap-2">
					{#each suggestions as suggestion (suggestion)}
						<Button
							on:click={() => submitSuggestion(suggestion)}
							variant="subtle"
							size="xs2"
							btnClasses="whitespace-normal text-center font-normal"
						>
							{suggestion}
						</Button>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</div>
