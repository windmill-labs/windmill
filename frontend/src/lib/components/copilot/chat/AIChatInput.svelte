<script lang="ts">
	import AppAvailableContextList from './AppAvailableContextList.svelte'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import ContextTextarea from './ContextTextarea.svelte'
	import AttachedFilesBar from './files/AttachedFilesBar.svelte'
	import autosize from '$lib/autosize'
	import type { AppDomSelectorElement, ContextElement } from './context'
	import { AIMode } from './AIChatManager.svelte'
	import { CHAT_INPUT_PADDING, getAiChatManager } from './aiChatManagerContext'
	import { formatMention } from './mention'
	import { twMerge } from 'tailwind-merge'
	import { tick, untrack, type Snippet } from 'svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import { ArrowUp, Loader2, Square, X } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { type PasteAttachment } from './pasteTokens'
	import { chatDraft, expanded } from './chatDraft'
	import {
		fileToAttachedImage,
		isImageFile,
		MAX_ATTACHED_IMAGES,
		MAX_IMAGE_BYTES,
		type AttachedImage
	} from './imageUtils'
	import { modelSupportsVision } from '../modelConfig'
	import { tryGetCurrentModel } from '$lib/aiStore'
	import ExpandableImage, {
		isImageViewerOpen
	} from '$lib/components/common/image/ExpandableImage.svelte'

	const aiChatManager = getAiChatManager()

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		isFirstMessage?: boolean
		disabled?: boolean
		placeholder?: string
		initialInstructions?: string
		initialPastes?: PasteAttachment[]
		initialImages?: AttachedImage[]
		editingMessageIndex?: number | null
		onEditEnd?: () => void
		className?: string
		onClickOutside?: () => void
		onSendRequest?: (instructions: string) => void
		showContext?: boolean
		bottomRightSnippet?: Snippet
		onKeyDown?: (e: KeyboardEvent) => void
		// When provided, overrides `aiChatManager.loading` for the send/stop
		// button — useful for callers driving their own request lifecycle
		// (e.g. the inline ⌘K widget runs requests outside the global
		// `aiChatManager.loading` flag).
		loading?: boolean
		// Called when the user clicks Stop. Defaults to `aiChatManager.cancel()`.
		onCancel?: () => void
		// Observe the composer draft as it changes (the text is local state —
		// `aiChatManager.instructions` only carries programmatic prompts). Used by
		// sessions to persist the typed-but-unsent prompt with the session draft.
		onDraftChange?: (text: string) => void
	}

	let {
		availableContext,
		selectedContext = $bindable([]),
		disabled = false,
		isFirstMessage = false,
		placeholder,
		initialInstructions = '',
		initialPastes = undefined,
		initialImages = undefined,
		editingMessageIndex = null,
		onEditEnd = () => {},
		className = '',
		onClickOutside = () => {},
		onSendRequest = undefined,
		showContext = true,
		bottomRightSnippet,
		onKeyDown = undefined,
		loading,
		onCancel,
		onDraftChange = undefined
	}: Props = $props()

	// GLOBAL-mode suggestion pool. We pick one at mount-time so each new
	// session lands on a different prompt; the choice stays stable for
	// the lifetime of this input so the placeholder doesn't shuffle as
	// the user is reading it.
	const GLOBAL_PLACEHOLDER_SUGGESTIONS = [
		'Write a hello-world flow',
		'Create a script that lists files in an S3 bucket',
		'Build a CRUD app for a customer table',
		'Schedule a daily cleanup of old runs',
		'Wrap an existing script into a flow with retries',
		'Add an HTTP trigger to an existing script',
		'Generate a report from a SQL query and email it',
		'Create a Postgres resource and a script that queries it',
		'Refactor a script to add error handling',
		'List my workspace flows and scripts'
	]
	const globalSuggestion =
		GLOBAL_PLACEHOLDER_SUGGESTIONS[
			Math.floor(Math.random() * GLOBAL_PLACEHOLDER_SUGGESTIONS.length)
		]

	// Generate mode-specific placeholder
	const modePlaceholder = $derived.by(() => {
		if (!isFirstMessage) {
			return 'Ask followup'
		}

		if (placeholder) {
			return placeholder
		}

		switch (aiChatManager.mode) {
			case AIMode.SCRIPT:
				return 'Modify this script...'
			case AIMode.FLOW:
				return 'Modify this flow...'
			case AIMode.APP:
				return 'Modify this app...'
			case AIMode.NAVIGATOR:
				return 'Navigate Windmill UI...'
			case AIMode.API:
				return 'Make API calls...'
			case AIMode.GLOBAL:
				return globalSuggestion
			case AIMode.ASK:
				return 'Ask questions about Windmill...'
			default:
				return 'Ask anything'
		}
	})

	let contextTextareaComponent: ContextTextarea | undefined = $state()
	let instructionsTextareaComponent: HTMLTextAreaElement | undefined = $state()
	let instructions = $state(untrack(() => initialInstructions))
	$effect(() => {
		const text = instructions
		untrack(() => onDraftChange?.(text))
	})
	// Collapsed big-paste blobs referenced by tokens in `instructions`.
	let pastes = $state<PasteAttachment[]>(untrack(() => initialPastes ?? []))
	// Per-message image attachments (drag/drop/paste), GLOBAL mode only. One-shot:
	// they attach to the next send and clear, unlike the persistent attached-files store.
	let images = $state<AttachedImage[]>(untrack(() => initialImages ?? []))
	// Images being decoded right now. Holds off sending so a message can never go
	// out without an attachment the user already dropped, and reserves cap slots
	// against a concurrent drop.
	let pendingImages = $state(0)

	/** Attach dropped/pasted image files (downscaled + bounded). GLOBAL mode only. */
	export async function addImages(files: (File | Blob)[]) {
		if (aiChatManager.mode !== AIMode.GLOBAL) return
		const imageFiles = files.filter(isImageFile)
		if (imageFiles.length === 0) return
		// tryGetCurrentModel returns undefined instead of throwing: this runs from a
		// drop/paste handler that can't surface a rejection.
		const model = tryGetCurrentModel()
		// Only known text-only models fail this, so attaching would certainly 400 the
		// next turn — refuse rather than warn and send it anyway.
		if (model && !modelSupportsVision(model.provider, model.model)) {
			sendUserToast(`${model.model} can't read images. Switch to a vision model first.`, true)
			return
		}
		// Count decodes already in flight: two drops that both read `images.length`
		// before either resolves would each claim the same free slots and overshoot
		// the cap.
		const remaining = MAX_ATTACHED_IMAGES - images.length - pendingImages
		if (remaining <= 0) {
			sendUserToast(`You can attach up to ${MAX_ATTACHED_IMAGES} images.`, true)
			return
		}
		const oversized = imageFiles.filter((f) => f.size > MAX_IMAGE_BYTES)
		if (oversized.length > 0) {
			const mb = Math.round(MAX_IMAGE_BYTES / 1_000_000)
			sendUserToast(`${oversized.length} image(s) over ${mb}MB were skipped.`, true)
		}
		const usable = imageFiles.filter((f) => f.size <= MAX_IMAGE_BYTES)
		if (usable.length === 0) return
		const batch = usable.slice(0, remaining)
		if (batch.length < usable.length) {
			sendUserToast(
				`You can attach up to ${MAX_ATTACHED_IMAGES} images; ${usable.length - batch.length} were skipped.`,
				true
			)
		}
		// Claim the slots before awaiting, and hold sending until they resolve:
		// decoding takes ~50-800ms, and a send during it would clear `images` while
		// this closure still appends to it, landing the picture on the next message.
		pendingImages += batch.length
		try {
			// One at a time: a decoded bitmap costs ~4 bytes per pixel (a 12MP photo is
			// ~48MB), so decoding the whole batch at once would hold every one of them
			// live simultaneously.
			const added: AttachedImage[] = []
			let failed = 0
			for (const file of batch) {
				try {
					added.push(await fileToAttachedImage(file))
				} catch {
					failed++
				}
			}
			if (added.length > 0) images = [...images, ...added]
			if (failed > 0) sendUserToast(`Could not attach ${failed} image(s).`, true)
		} finally {
			pendingImages -= batch.length
		}
	}

	function removeImage(index: number) {
		images = images.filter((_, i) => i !== index)
	}

	// App mode @ mention state
	let showAppContextTooltip = $state(false)
	let appContextTooltipWord = $state('')
	let appTooltipPosition = $state({ x: 0, y: 0 })
	let appTooltipElement = $state<HTMLDivElement | undefined>(undefined)
	let appTooltipCurrentViewNumber = $state(0)

	// Modes that show the rich textarea with @-context support (workspace
	// scripts, workspace flows, code blocks, DBs, etc.).
	const isContextEnabledMode = $derived(
		aiChatManager.mode === AIMode.SCRIPT ||
			aiChatManager.mode === AIMode.FLOW ||
			aiChatManager.mode === AIMode.GLOBAL
	)

	const domSelectorChips = $derived(
		(selectedContext ?? []).filter((c): c is AppDomSelectorElement => c.type === 'app_dom_selector')
	)

	// DOM selector chips can share a display title (two `button.btn` from repeated
	// elements), so identify them by (appPath, selector) — keying or removing by
	// title would collide, giving repeated chips one Svelte key and deleting them
	// together. Other context types stay identified by (type, title).
	function contextKey(c: ContextElement): string {
		return c.type === 'app_dom_selector' ? `dom:${c.appPath}:${c.selector}` : `${c.type}:${c.title}`
	}
	function isSameContextElement(a: ContextElement, b: ContextElement): boolean {
		if (a.type !== b.type) return false
		if (a.type === 'app_dom_selector' && b.type === 'app_dom_selector') {
			return a.selector === b.selector && a.appPath === b.appPath
		}
		return a.title === b.title
	}

	/** Append `@title` to the textarea so the button-picker path stays in
	 * sync with the inline `@<word>` mention path — both leave a visible
	 * token tied to the selectedContext entry, which the textarea diffs on
	 * to auto-remove items when the user deletes them. No-op when the
	 * mention is already present so re-picking the same item doesn't
	 * leave duplicate tokens. */
	export function insertMention(title: string) {
		const target = `@${title}`
		if (instructions.split(/\s+/).includes(target)) return
		const sep = instructions.length === 0 || /\s$/.test(instructions) ? '' : ' '
		instructions = `${instructions}${sep}${target} `
	}

	/** Strip every `@title` token from the textarea — used when the user
	 * deletes the corresponding badge so the badge X-button mirrors the
	 * inverse (text-delete-to-badge-remove) sync. Only matches `@title` as a
	 * standalone token (boundary on both sides) so substring matches don't
	 * bleed into other words; only the whitespace adjacent to the removed
	 * mention is collapsed so unrelated double-spaces stay intact. */
	export function removeMention(title: string) {
		// Pre-zap the textarea's mention diff snapshot so the upcoming strip
		// doesn't refire the removal effect on a same-title sibling — the host
		// has already mutated `selectedContext` to drop the targeted entry.
		contextTextareaComponent?.unsyncMention(title)
		const escaped = title.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
		const re = new RegExp(`(^|\\s)@${escaped}(\\s|$)`, 'g')
		instructions = instructions.replace(re, (_m, lead, trail) => {
			// Boundary on at least one side → drop the mention entirely.
			if (!lead || !trail) return ''
			// Middle of text: keep ONE of the bracketing whitespace chars so
			// the surviving tokens are still separated; preserve the leading
			// one verbatim so newlines/tabs aren't downgraded to spaces.
			return lead
		})
	}

	export function focusInput() {
		if (isContextEnabledMode) {
			contextTextareaComponent?.focus()
		} else {
			instructionsTextareaComponent?.focus()
		}
	}

	// Restore composer contents after a rolled-back turn. No-op when the user
	// already drafted something new — typed text or attached images (including
	// ones still decoding) — restoring would clobber it.
	/** Returns whether the restore was taken: an occupied composer keeps its own
	 * draft and declines, and the caller must then leave that draft's context
	 * alone too — restoring context for text that was dropped would retarget the
	 * draft the user is still writing. */
	export function restoreInstructions(
		value: string,
		restoredPastes: PasteAttachment[] = [],
		restoredImages: AttachedImage[] = []
	): boolean {
		if (instructions.trim() || images.length > 0 || pendingImages > 0) return false
		instructions = value
		pastes = restoredPastes
		images = restoredImages
		focusInput()
		return true
	}

	/** Put text back into the textarea (queued-message delete, or restore
	 * after a cancelled/errored turn), prepended to any draft so nothing
	 * the user typed is lost. Restored images join whatever is already
	 * attached, up to the cap — dropping them would lose the attachment
	 * silently, which is the whole reason the queue carries them. */
	export function prependText(text: string, restoredImages: AttachedImage[] = []): boolean {
		// Whether the restored text landed on top of a draft the user was already
		// writing: both instructions now share one composer, so the caller must keep
		// both their contexts rather than replacing one with the other.
		const mergedIntoDraft = !!text && !!instructions.trim()
		// An image-only restore has empty text; prepending it would only add blank lines.
		if (text) {
			instructions = instructions.trim() ? `${text}\n\n${instructions}` : text
		}
		if (restoredImages.length > 0) {
			const merged = [...images, ...restoredImages]
			if (merged.length > MAX_ATTACHED_IMAGES) {
				sendUserToast(
					`You can attach up to ${MAX_ATTACHED_IMAGES} images; ${merged.length - MAX_ATTACHED_IMAGES} restored image(s) were dropped.`,
					true
				)
			}
			images = merged.slice(0, MAX_ATTACHED_IMAGES)
		}
		focusInput()
		return mergedIntoDraft
	}

	/** Insert a plain @filename mention for an attached file (used by the @ menu Files category). */
	export function insertFileMention(name: string) {
		const sep = instructions.length === 0 || instructions.endsWith(' ') ? '' : ' '
		instructions = `${instructions}${sep}${formatMention(name)} `
		focusInput()
	}

	function clickOutside(node: HTMLElement) {
		function handleClick(event: MouseEvent) {
			// An expanded image chip renders in a portal, so clicks in it land outside
			// this node without being outside the composer. Dismissing on them would
			// discard the edit the user opened the image from.
			if (isImageViewerOpen()) return
			if (node && !node.contains(event.target as Node)) {
				onClickOutside()
			}
		}

		document.addEventListener('click', handleClick, true)
		return {
			destroy() {
				document.removeEventListener('click', handleClick, true)
			}
		}
	}

	export async function addContextToSelection(contextElement: ContextElement) {
		if (!selectedContext || !availableContext) return

		const alreadySelected = selectedContext.find(
			(c) => c.type === contextElement.type && c.title === contextElement.title
		)
		if (alreadySelected) return

		// Workspace items are fetched on-demand and not in availableContext,
		// so skip the availableContext check for them
		const isWorkspaceItem =
			contextElement.type === 'workspace_script' ||
			contextElement.type === 'workspace_flow' ||
			contextElement.type === 'workspace_app'
		if (
			!isWorkspaceItem &&
			!availableContext.find(
				(c) => c.type === contextElement.type && c.title === contextElement.title
			)
		) {
			return
		}

		let contextToAdd = contextElement

		if (
			contextElement.type === 'app_datatable' &&
			aiChatManager.mode === AIMode.APP &&
			aiChatManager.appAiChatHelpers
		) {
			const appAiChatHelpers = aiChatManager.appAiChatHelpers
			appAiChatHelpers.addTableToWhitelist(
				contextElement.datatableName,
				contextElement.schemaName,
				contextElement.tableName
			)

			if (!contextElement.columns) {
				try {
					contextToAdd = {
						...contextElement,
						columns: await appAiChatHelpers.getDatatableTableSchema(
							contextElement.datatableName,
							contextElement.schemaName,
							contextElement.tableName
						)
					}
				} catch (e) {
					console.error('Failed to load datatable table schema:', e)
					sendUserToast(
						'Failed to load datatable table schema',
						true,
						[],
						e instanceof Error ? e.message : String(e)
					)
				}
			}
		}

		const duplicateAfterAwait = selectedContext.find(
			(c) => c.type === contextToAdd.type && c.title === contextToAdd.title
		)
		if (duplicateAfterAwait) return

		selectedContext = [...selectedContext, contextToAdd]
	}

	function sendRequest() {
		// The send button is disabled while decoding, but Enter reaches here directly.
		// Sending now would drop the in-flight images onto the following message.
		if (pendingImages > 0) {
			return
		}
		if (aiChatManager.loading) {
			// Queue the message instead of silently discarding it — it is
			// auto-sent when the streaming turn completes successfully.
			// Editing-while-loading keeps the old discard behavior. Paste
			// tokens are expanded into the queued text (the queue is plain
			// strings), so the full content survives the auto-send.
			if (editingMessageIndex === null && (instructions.trim() || images.length > 0)) {
				aiChatManager.queueMessage(expanded(chatDraft(instructions, pastes)), images)
				contextTextareaComponent?.clearForSend()
				instructions = ''
				pastes = []
				images = []
			}
			return
		}
		if (editingMessageIndex !== null) {
			// In edit mode selectedContext is the edit box's own copy (seeded from the
			// message's original chips), so send exactly what's shown — the user may
			// have added or removed chips.
			aiChatManager.restartGeneration(
				editingMessageIndex,
				instructions,
				pastes,
				images,
				selectedContext
			)
			onEditEnd()
		} else {
			aiChatManager.sendRequest({ instructions, pastes, images })
			// clearForSend() pre-zaps the textarea's mention-sync so the wipe
			// doesn't drop `selectedContext` before `AIChatManager.beforeSend`
			// snapshots it. Only mounted in SCRIPT/FLOW/GLOBAL — APP and the
			// fallback textarea still rely on the plain `instructions = ''`
			// reset (no `@`-mention state to coordinate).
			contextTextareaComponent?.clearForSend()
			instructions = ''
			pastes = []
			images = []
		}
	}

	// A custom `onSendRequest` consumer (e.g. the inline ⌘K widget) has no chip
	// display, so it gets the fully expanded text; the default path keeps tokens
	// for the conversation bubble and expands them for the LLM inside the manager.
	function submitRequest() {
		if (onSendRequest) {
			onSendRequest(expanded(chatDraft(instructions, pastes)))
		} else {
			sendRequest()
		}
	}

	$effect(() => {
		if (editingMessageIndex !== null) {
			focusInput()
		}
	})

	// Properties to copy for caret position calculation (app mode)
	const caretProperties = [
		'direction',
		'boxSizing',
		'width',
		'height',
		'overflowX',
		'overflowY',
		'borderTopWidth',
		'borderRightWidth',
		'borderBottomWidth',
		'borderLeftWidth',
		'borderStyle',
		'paddingTop',
		'paddingRight',
		'paddingBottom',
		'paddingLeft',
		'fontStyle',
		'fontVariant',
		'fontWeight',
		'fontStretch',
		'fontSize',
		'fontSizeAdjust',
		'lineHeight',
		'fontFamily',
		'textAlign',
		'textTransform',
		'textIndent',
		'textDecoration',
		'letterSpacing',
		'wordSpacing',
		'tabSize',
		'MozTabSize'
	]

	function getCaretCoordinates(element: HTMLTextAreaElement, position: number) {
		const div = document.createElement('div')
		div.id = 'input-textarea-caret-position-mirror-div'
		document.body.appendChild(div)

		const style = div.style
		const computed = window.getComputedStyle(element)
		const isInput = element.nodeName === 'INPUT'

		style.whiteSpace = 'pre-wrap'
		if (!isInput) style.wordWrap = 'break-word'

		style.position = 'absolute'
		style.visibility = 'hidden'

		caretProperties.forEach(function (prop) {
			if (isInput && prop === 'lineHeight') {
				if (computed.boxSizing === 'border-box') {
					const height = parseInt(computed.height)
					const outerHeight =
						parseInt(computed.paddingTop) +
						parseInt(computed.paddingBottom) +
						parseInt(computed.borderTopWidth) +
						parseInt(computed.borderBottomWidth)
					const targetHeight = outerHeight + parseInt(computed.lineHeight)
					if (height > targetHeight) {
						style.lineHeight = height - outerHeight + 'px'
					} else if (height === targetHeight) {
						style.lineHeight = computed.lineHeight
					} else {
						style.lineHeight = '0'
					}
				} else {
					style.lineHeight = computed.height
				}
			} else {
				style[prop] = computed[prop]
			}
		})

		const isFirefox =
			(window as typeof window & { mozInnerScreenX: number }).mozInnerScreenX != null
		if (isFirefox) {
			if (element.scrollHeight > parseInt(computed.height)) style.overflowY = 'scroll'
		} else {
			style.overflow = 'hidden'
		}

		div.textContent = element.value.substring(0, position)

		if (isInput) div.textContent = div.textContent.replace(/\s/g, '\u00a0')

		const span = document.createElement('span')
		span.textContent = element.value.substring(position) || '.'
		div.appendChild(span)

		const coordinates = {
			top: span.offsetTop + parseInt(computed['borderTopWidth']),
			left: span.offsetLeft + parseInt(computed['borderLeftWidth']),
			height: parseInt(computed['lineHeight'])
		}

		document.body.removeChild(div)

		return coordinates
	}

	async function updateAppTooltipPosition(currentViewItemsNumber: number) {
		if (!instructionsTextareaComponent) return

		try {
			const coords = getCaretCoordinates(
				instructionsTextareaComponent,
				instructionsTextareaComponent.selectionEnd
			)
			const rect = instructionsTextareaComponent.getBoundingClientRect()

			const itemHeight = 28
			const containerPadding = 8
			const maxHeight = 192 + containerPadding

			const numItems = currentViewItemsNumber
			let uncappedHeight =
				numItems > 0 ? numItems * itemHeight - 4 + containerPadding : containerPadding
			uncappedHeight = Math.max(uncappedHeight, containerPadding)

			const estimatedTooltipHeight = Math.min(uncappedHeight, maxHeight)
			const margin = 6

			let finalX = rect.left + coords.left - 70
			let finalY: number

			if (isFirstMessage) {
				finalY = rect.top + coords.top + coords.height - 3
			} else {
				finalY = rect.top + coords.top - estimatedTooltipHeight - margin
			}

			appTooltipPosition = {
				x: finalX,
				y: finalY
			}

			await tick()

			if (appTooltipElement) {
				const tooltipRect = appTooltipElement.getBoundingClientRect()
				const tooltipWidth = tooltipRect.width

				if (finalX + tooltipWidth > window.innerWidth) {
					finalX = Math.max(10, window.innerWidth - tooltipWidth - 10)

					appTooltipPosition = {
						x: finalX,
						y: finalY
					}
				}
			}
		} catch (error) {
			console.error('Error updating tooltip position', error)
			showAppContextTooltip = false
		}
	}

	function handleAppInput(_e: Event) {
		const words = instructions.split(/\s+/)
		const lastWord = words[words.length - 1]

		if (
			lastWord.startsWith('@') &&
			(!availableContext.find((c) => c.title === lastWord.slice(1)) ||
				!selectedContext.find((c) => c.title === lastWord.slice(1)))
		) {
			showAppContextTooltip = true
			appContextTooltipWord = lastWord
		} else {
			showAppContextTooltip = false
			appContextTooltipWord = ''
		}
	}

	function handleAppContextSelection(contextElement: ContextElement) {
		void addContextToSelection(contextElement)
		// Update instructions with the selected context title
		const index = instructions.lastIndexOf('@')
		if (index !== -1) {
			instructions = instructions.substring(0, index) + `@${contextElement.title}`
		}
		showAppContextTooltip = false
	}

	$effect(() => {
		if (showAppContextTooltip) {
			updateAppTooltipPosition(appTooltipCurrentViewNumber)
		}
	})
</script>

{#snippet sendStopButton()}
	{@const isLoading = loading ?? aiChatManager.loading}
	{@const emptyDraft = instructions.trim().length === 0 && images.length === 0}
	<!-- A text-free GLOBAL draft with context chips is a valid turn (Enter
	     already sends it), so the button stays enabled there for pointer/touch
	     parity — mirrors the sendRequest guard. Custom onSendRequest consumers
	     (inline ⌘K) and editor copilots need content. -->
	{@const sendDisabled =
		disabled ||
		pendingImages > 0 ||
		(emptyDraft &&
			(onSendRequest !== undefined ||
				aiChatManager.mode !== AIMode.GLOBAL ||
				selectedContext.length === 0))}
	<Button
		variant="subtle"
		unifiedSize="md"
		iconOnly
		title={isLoading ? 'Stop' : 'Send'}
		startIcon={{ icon: isLoading ? Square : ArrowUp }}
		disabled={!isLoading && sendDisabled}
		on:click={() => {
			if (isLoading) {
				onCancel ? onCancel() : aiChatManager.cancel()
			} else if (!sendDisabled) {
				submitRequest()
			}
		}}
	/>
{/snippet}

{#snippet contextPickerRow()}
	{#if selectedContext.length > 0}
		<div class="flex flex-row flex-wrap items-center gap-1 px-2.5 pt-2">
			{#each selectedContext as element (contextKey(element))}
				<ContextElementBadge
					contextElement={element}
					deletable
					onDelete={() => {
						selectedContext = selectedContext?.filter((c) => !isSameContextElement(c, element))
						removeMention(element.title)
					}}
				/>
			{/each}
		</div>
	{/if}
{/snippet}

<!-- DOM selector chips (raw-app inspector picks) always show — even in GLOBAL/
     session mode where the general context picker row is hidden (showContext=false). -->
{#snippet domSelectorChipRow()}
	{#if domSelectorChips.length > 0}
		<div class="flex flex-row flex-wrap items-center gap-1 px-2.5 pt-2">
			{#each domSelectorChips as element (contextKey(element))}
				<ContextElementBadge
					contextElement={element}
					deletable
					onDelete={() => {
						selectedContext = selectedContext?.filter((c) => !isSameContextElement(c, element))
					}}
				/>
			{/each}
		</div>
	{/if}
{/snippet}

{#snippet imageChipsRow()}
	{#if images.length > 0 || pendingImages > 0}
		<div class="flex flex-row flex-wrap items-center gap-1.5 mb-1">
			{#each images as image, i (i)}
				<div class="relative group">
					<!-- The chip is a 48px object-cover crop, so the expanded view is the only
					     way to check what was actually attached before sending it. -->
					<ExpandableImage
						src={image.dataUrl}
						alt={image.name ?? 'attached image'}
						class="h-12 w-12 object-cover rounded border border-border-light"
					/>
					<button
						type="button"
						title="Remove image"
						class="absolute -top-1.5 -right-1.5 bg-surface-secondary border border-border-light rounded-full p-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
						onclick={() => removeImage(i)}
					>
						<X size={10} />
					</button>
				</div>
			{/each}
			<!-- Placeholders for images still being decoded. Sending is held until they
			     land, so the row has to show that something is on its way. -->
			{#each { length: pendingImages } as _, i (i)}
				<div
					class="h-12 w-12 rounded border border-border-light bg-surface-secondary flex items-center justify-center"
					title="Preparing image..."
				>
					<Loader2 size={14} class="animate-spin text-tertiary" />
				</div>
			{/each}
		</div>
	{/if}
{/snippet}

<div
	use:clickOutside
	class="relative mt-1"
	role="presentation"
	onkeydown={(e) => {
		if (e.key === 'Escape' && aiChatManager.loading) {
			e.preventDefault()
			aiChatManager.cancel()
		}
	}}
>
	{#if isContextEnabledMode}
		<div class="relative">
			<ContextTextarea
				bind:this={contextTextareaComponent}
				bind:value={instructions}
				bind:pastes
				onImageFiles={aiChatManager.mode === AIMode.GLOBAL
					? (files) => void addImages(files)
					: undefined}
				{availableContext}
				{selectedContext}
				placeholder={modePlaceholder}
				onAddContext={(contextElement) => void addContextToSelection(contextElement)}
				onRemoveContext={(element) => {
					selectedContext = selectedContext?.filter((c) => !isSameContextElement(c, element))
				}}
				onSendRequest={() => {
					if (disabled) {
						return
					}
					submitRequest()
				}}
				{disabled}
				{onKeyDown}
			>
				{#snippet leading()}
					{#if aiChatManager.mode === AIMode.GLOBAL}
						<div class="px-2.5 empty:hidden">
							<AttachedFilesBar />
						</div>
					{/if}
					{#if showContext}
						{@render contextPickerRow()}
					{:else}
						{@render domSelectorChipRow()}
					{/if}
					{@render imageChipsRow()}
				{/snippet}
			</ContextTextarea>
			{#if !bottomRightSnippet}
				<div class="absolute bottom-1 right-1">
					{@render sendStopButton()}
				</div>
			{/if}
		</div>
	{:else if aiChatManager.mode === AIMode.APP}
		{#if showContext}
			{@render contextPickerRow()}
		{/if}
		<div class={twMerge('relative w-full scroll-pb-2', className)}>
			<textarea
				bind:this={instructionsTextareaComponent}
				bind:value={instructions}
				use:autosize={{ maxHeight: '40vh' }}
				oninput={handleAppInput}
				onblur={() => {
					setTimeout(() => {
						showAppContextTooltip = false
					}, 200)
				}}
				onkeydown={(e) => {
					if (onKeyDown) {
						onKeyDown(e)
					}
					if (showAppContextTooltip) {
						// avoid new line after Enter in the tooltip
						if (e.key === 'Enter') {
							e.preventDefault()
						}
						return
					}
					if (e.key === 'Enter' && !e.shiftKey) {
						e.preventDefault()
						sendRequest()
					}
				}}
				rows={1}
				placeholder={modePlaceholder}
				class={twMerge('resize-none', CHAT_INPUT_PADDING)}
				{disabled}
			></textarea>
			{#if !bottomRightSnippet}
				<div class="absolute bottom-1 right-1">
					{@render sendStopButton()}
				</div>
			{/if}
		</div>
		{#if showAppContextTooltip}
			<Portal target="body">
				<div
					bind:this={appTooltipElement}
					class="absolute bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg"
					style="left: {appTooltipPosition.x}px; top: {appTooltipPosition.y}px; z-index: {zIndexes.tooltip};"
				>
					<AppAvailableContextList
						{availableContext}
						{selectedContext}
						onSelect={(element) => {
							handleAppContextSelection(element)
						}}
						showAllAvailable={true}
						stringSearch={appContextTooltipWord.slice(1)}
						onViewChange={(newNumber) => {
							appTooltipCurrentViewNumber = newNumber
						}}
						setShowing={(showing) => {
							showAppContextTooltip = showing
						}}
					/>
				</div>
			</Portal>
		{/if}
	{:else}
		<div class={twMerge('relative w-full scroll-pb-2 pt-2', className)}>
			<textarea
				bind:this={instructionsTextareaComponent}
				bind:value={instructions}
				use:autosize={{ maxHeight: '40vh' }}
				onkeydown={(e) => {
					if (onKeyDown) {
						onKeyDown(e)
					}
					if (e.key === 'Enter' && !e.shiftKey) {
						e.preventDefault()
						sendRequest()
					}
				}}
				rows={1}
				placeholder={modePlaceholder}
				class={twMerge('resize-none', CHAT_INPUT_PADDING)}
				{disabled}
			></textarea>
			{#if !bottomRightSnippet}
				<div class="absolute bottom-1 right-1">
					{@render sendStopButton()}
				</div>
			{/if}
		</div>
	{/if}
	{#if bottomRightSnippet}
		<div class="absolute bottom-1 right-1">
			{@render bottomRightSnippet()}
		</div>
	{/if}
</div>
