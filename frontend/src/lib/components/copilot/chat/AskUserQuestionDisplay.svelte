<script lang="ts">
	import { onMount, tick } from 'svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import { CircleHelp, ArrowUp, Plus, Square, SquareCheck } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import type { UserQuestionDisplay } from './shared'

	// Sessions inject a per-pane `AIChatManager` via context; outside of
	// sessions getAiChatManager falls back to the global singleton. Without
	// this, answers clicked inside a session would dispatch to the singleton's
	// pending callbacks map (which doesn't have the session manager's question
	// callback), and the AI loop would stall.
	const aiChatManager = getAiChatManager()

	interface Props {
		toolCallId: string
		userQuestion: UserQuestionDisplay
	}

	let { toolCallId, userQuestion }: Props = $props()

	const multiSelect = $derived(userQuestion.multiSelect === true)

	// Multi-select choices accumulate here until the user explicitly submits — a
	// SvelteSet (not a plain Set) so `.add()/.delete()` drive Svelte reactivity.
	let picked = new SvelteSet<string>()

	// Surface picked custom answers as their own toggle rows so the user can see
	// and un-pick them before submitting; a value equal to a proposed choice just
	// checks that choice rather than adding a duplicate row.
	const customPicked = $derived(
		multiSelect ? [...picked].filter((p) => !userQuestion.choices.includes(p)) : []
	)
	const displayedChoices = $derived([...userQuestion.choices, ...customPicked])

	let cardNode = $state<HTMLDivElement | undefined>()
	let choiceButtons = $state<(HTMLButtonElement | undefined)[]>([])
	let submitButton = $state<HTMLButtonElement | undefined>()
	let customAnswerInput = $state<TextInput | undefined>()
	let customAnswer = $state('')
	let canSubmitCustomAnswer = $derived(customAnswer.trim().length > 0)

	// Submit joins the roving cursor only while it is actionable: a disabled
	// Button carries tabindex=-1, so an empty-selection Submit can't take focus
	// and would strand arrow-key navigation on it. Excluding it from itemCount
	// when unreachable keeps the wraparound landing on real, focusable stops.
	const submitReachable = $derived(multiSelect && picked.size > 0)

	// The custom-answer input is the next stop in the roving cursor after all
	// (proposed + custom) choice rows; Submit follows it when reachable.
	const customAnswerIndex = $derived(displayedChoices.length)
	const submitIndex = $derived(submitReachable ? displayedChoices.length + 1 : -1)
	const itemCount = $derived(displayedChoices.length + 1 + (submitReachable ? 1 : 0))
	// The item currently under the keyboard cursor. In single mode it is mirrored
	// onto the matching choice's `selected` prop for the highlight; in multi mode
	// `selected` is reserved for checked state, so the cursor shows as a focus ring.
	let activeIndex = $state(0)

	onMount(() => {
		void tick().then(() => {
			// preventScroll: a scrolling focus() strands the taller multi-select card's
			// Submit below the fold. The container auto-scroll is a stick-to-bottom that
			// stops once the user scrolls up, so reveal the card itself instead.
			focusIndex(0, { preventScroll: true })
			cardNode?.scrollIntoView({ block: 'nearest' })
		})
	})

	function focusIndex(index: number, opts?: FocusOptions) {
		activeIndex = index
		if (index === customAnswerIndex) {
			customAnswerInput?.focus()
		} else if (index === submitIndex) {
			submitButton?.focus(opts)
		} else {
			choiceButtons[index]?.focus(opts)
		}
	}

	// Cmd/Ctrl+Enter finalizes a multi-select answer from anywhere in the card.
	function isSubmitShortcut(event: KeyboardEvent): boolean {
		return event.key === 'Enter' && (event.metaKey || event.ctrlKey)
	}

	function selectChoice(choice: string) {
		if (multiSelect) {
			if (picked.has(choice)) {
				picked.delete(choice)
			} else {
				picked.add(choice)
			}
			return
		}
		aiChatManager.handleUserQuestionAnswer(toolCallId, [choice])
	}

	function submitPicked() {
		if (!multiSelect || picked.size === 0) {
			return
		}
		aiChatManager.handleUserQuestionAnswer(toolCallId, [...picked])
	}

	function submitCustomAnswer() {
		const answer = customAnswer.trim()
		if (!answer) {
			return
		}

		if (multiSelect) {
			// Clear after queuing so the user can add more answers before Submit.
			picked.add(answer)
			customAnswer = ''
			// The list grew under the still-focused input, shifting its roving index;
			// resync so a later background-area click refocuses the input, not the new row.
			activeIndex = customAnswerIndex
			return
		}

		aiChatManager.handleUserQuestionAnswer(toolCallId, [answer])
	}

	function handleChoiceKeydown(event: KeyboardEvent, choice: string, index: number) {
		if (multiSelect && isSubmitShortcut(event)) {
			event.preventDefault()
			event.stopPropagation()
			submitPicked()
			return
		}

		if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((index + 1) % itemCount)
			return
		}

		if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((index - 1 + itemCount) % itemCount)
			return
		}

		if (event.key === 'Enter') {
			event.preventDefault()
			event.stopPropagation()
			// A custom row is only present while picked, so toggling it here removes
			// it and unmounts the button — redirect focus to a stable stop so the
			// keyboard flow survives. Proposed choices stay mounted when unchecked.
			const removesRow = customPicked.includes(choice)
			selectChoice(choice)
			if (removesRow) {
				void tick().then(() => focusIndex(customAnswerIndex))
			}
		}
	}

	// Clicking the card's empty area must pull focus to the active item; otherwise
	// focus stays on the chat's scroll container and arrow keys scroll the
	// conversation instead of moving the selection. Wired as an action rather than
	// a declarative on:click so the non-interactive card needs no keyboard handler,
	// and on `click` rather than `pointerdown` so click-dragging to select text
	// still works.
	function focusActiveOnBackgroundClick(node: HTMLElement) {
		function onClick(event: MouseEvent) {
			const interactive = (event.target as HTMLElement | null)?.closest(
				'button, input, textarea, a, [contenteditable]'
			)
			// A control inside the card (a choice, the answer input, the send button)
			// manages its own focus — leave it alone. `closest` can also match the
			// message-row button the chat wraps every message in, which is an ancestor
			// of the card, so only bail out when the match is actually inside the card.
			if (interactive && node.contains(interactive)) {
				return
			}
			event.stopPropagation()
			focusIndex(activeIndex)
		}
		node.addEventListener('click', onClick)
		return {
			destroy() {
				node.removeEventListener('click', onClick)
			}
		}
	}

	function handleCustomAnswerKeydown(event: KeyboardEvent) {
		if (multiSelect && isSubmitShortcut(event)) {
			event.preventDefault()
			event.stopPropagation()
			submitPicked()
			return
		}

		// Only ArrowUp/ArrowDown roam out of the input — Left/Right stay free for
		// moving the text caret within the answer.
		if (event.key === 'ArrowUp') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((customAnswerIndex - 1 + itemCount) % itemCount)
			return
		}

		if (event.key === 'ArrowDown') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((customAnswerIndex + 1) % itemCount)
			return
		}

		if (event.key === 'Enter') {
			event.preventDefault()
			event.stopPropagation()
			submitCustomAnswer()
		}
	}

	function handleSubmitKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((submitIndex + 1) % itemCount)
			return
		}

		if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((submitIndex - 1 + itemCount) % itemCount)
			return
		}

		if (event.key === 'Enter') {
			event.preventDefault()
			event.stopPropagation()
			submitPicked()
		}
	}
</script>

<!-- scroll-mb clears the chat's sticky "Waiting for your input" chip (pinned at the
     viewport bottom) so the mount scrollIntoView leaves the Submit button uncovered. -->
<div
	bind:this={cardNode}
	class="scroll-mb-8 rounded-md border border-border-light bg-surface p-3"
	data-chat-keyboard-scope="ask-user-question"
	use:focusActiveOnBackgroundClick
>
	<div class="flex items-start gap-2">
		<CircleHelp class="h-4 w-4 shrink-0 text-accent" />
		<p class="min-w-0 flex-1 whitespace-pre-wrap text-xs font-semibold text-emphasis"
			>{userQuestion.question}</p
		>
	</div>

	<div class="mt-3 flex flex-col gap-2">
		{#each displayedChoices as choice, index (index)}
			<Button
				variant="default"
				unifiedSize="sm"
				selected={multiSelect ? picked.has(choice) : activeIndex === index}
				bind:element={choiceButtons[index]}
				onClick={() => selectChoice(choice)}
				onkeydown={(event) => handleChoiceKeydown(event, choice, index)}
				onfocus={() => (activeIndex = index)}
				btnClasses={`h-auto min-h-7 justify-start whitespace-normal py-1.5 text-left ${
					multiSelect ? '' : 'focus-visible:!ring-0 focus-visible:!outline-none'
				}`}
			>
				{#if multiSelect}
					<!-- Show an empty box on every row, not only when checked, so multi-select is obvious up front. -->
					{#if picked.has(choice)}
						<SquareCheck class="mr-2 h-4 w-4 shrink-0" />
					{:else}
						<Square class="mr-2 h-4 w-4 shrink-0 text-secondary" />
					{/if}
				{/if}
				<span class="break-words">{choice}</span>
			</Button>
		{/each}

		<div class="flex min-w-0 gap-2 pt-1">
			<TextInput
				bind:this={customAnswerInput}
				bind:value={customAnswer}
				class="min-w-0 flex-1"
				size="sm"
				inputProps={{
					'aria-label': 'Custom answer',
					placeholder: 'Custom answer',
					onkeydown: handleCustomAnswerKeydown,
					onfocus: () => (activeIndex = customAnswerIndex)
				}}
			/>
			<Button
				variant="subtle"
				unifiedSize="sm"
				iconOnly
				title={multiSelect ? 'Add answer' : 'Send'}
				startIcon={{ icon: multiSelect ? Plus : ArrowUp }}
				disabled={!canSubmitCustomAnswer}
				onClick={submitCustomAnswer}
				btnClasses="shrink-0"
			/>
		</div>

		{#if multiSelect}
			<Button
				variant="accent"
				unifiedSize="sm"
				bind:element={submitButton}
				disabled={picked.size === 0}
				onClick={submitPicked}
				onkeydown={handleSubmitKeydown}
				onfocus={() => (activeIndex = submitIndex)}
				btnClasses="mt-1"
			>
				Submit{picked.size > 0 ? ` (${picked.size})` : ''}
			</Button>
		{/if}
	</div>
</div>
