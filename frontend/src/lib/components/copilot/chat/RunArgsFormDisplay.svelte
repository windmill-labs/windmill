<script lang="ts">
	import { onMount, tick, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { Play, X } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { processSecretArgs } from '$lib/components/secretArgUtils'
	import { enforceDisabledDefaults, resetKeysToast } from '$lib/components/job_args'
	import { sendUserToast } from '$lib/utils'
	import { getAiChatManager } from './aiChatManagerContext'
	import { PLAN_MODE_MESSAGES } from './planModeMessages'
	import { scrollFades } from './scrollFades.svelte'
	import type { RunFormDisplay } from './shared'

	// Never the imported singleton: submitting has to resolve the pending callback of
	// the manager that opened this form, which in a session is a per-pane one.
	const aiChatManager = getAiChatManager()

	interface Props {
		toolCallId: string
		runForm: RunFormDisplay
		/** `card` caps its own height inside the chat transcript; `pane` fills the preview
		 * tab it was opened into and lets the fields take the whole height. */
		layout?: 'card' | 'pane'
	}

	let { toolCallId, runForm, layout = 'card' }: Props = $props()

	// The chat's workspace, not the globally-active one: a session may be acting on a
	// fork, and that is where the job runs — so the pickers and the ephemeral secret
	// variables have to resolve there too.
	const workspace = $derived(aiChatManager.operatingWorkspace)

	// The manager's draft, not a copy of its own: the chat card and the preview pane are two
	// views of one form, and moving between them has to keep what was typed. untrack because
	// the message is replaced on every patch to the card, and re-seeding from a later copy of
	// it would discard those edits.
	const draft = untrack(() => aiChatManager.runFormDraft(toolCallId, runForm))

	const properties = $derived(draft.schema?.properties ?? {})
	const hasArgs = $derived(Object.keys(properties).length > 0)

	let isValid = $state(true)
	let submitting = $state(false)
	let cardNode = $state<HTMLDivElement | undefined>()

	// The picker moves while a form sits open, so this is live state, not mount-time. Both
	// writes the form makes on its own are held off it — the variable a password field
	// mints on its first keystroke, the entrypoint a `dynselect-` argument runs — because
	// neither waits for Run, and plan mode promised neither would happen.
	const planMode = $derived(aiChatManager.planModeActive)

	onMount(() => {
		void tick().then(() => cardNode?.scrollIntoView({ block: 'nearest' }))
	})

	const fades = scrollFades()
	const { container: fadeContainer, content: fadeContent, measure: measureFades } = fades
	// The two hosts stand on different surfaces — the chat card on the tool call's own, the
	// preview tab on the raised one — and a fade has to end in the colour behind it.
	const fadeTo = $derived(
		layout === 'pane'
			? 'bg-gradient-to-t from-surface-tertiary via-surface-tertiary/60 to-transparent'
			: 'bg-gradient-to-t from-surface via-surface/60 to-transparent'
	)

	async function run() {
		if (submitting || !isValid) return
		// Before processSecretArgs, not after: a card restored from history outlives the
		// manager that opened it, so submitting would mint an ephemeral secret variable
		// per click and still run nothing.
		if (!aiChatManager.isRunFormPending(toolCallId)) {
			sendUserToast('This run form is no longer active — ask again to run the script.', true)
			return
		}
		// Ahead of processSecretArgs, which writes ephemeral variables to the workspace: the
		// autonomy picker moves while a form sits pending, and the re-gate on the other side
		// of the callback runs too late to unmake a write plan mode promised not to do.
		if (aiChatManager.planModeActive) {
			sendUserToast(PLAN_MODE_MESSAGES.runFormRefused, true)
			return
		}
		submitting = true
		// Only the disabled fields, and only because `RunForm` does the same before its own
		// run: what the card showed is otherwise what runs. Re-filtering it here would delete
		// the user's own typing between Run and the job — a free-form field the form gave a
		// JSON editor to holds keys no schema names, and they are still theirs.
		const { args: enforced, resetKeys } = enforceDisabledDefaults(draft.args ?? {}, draft.schema)
		if (resetKeys.length > 0) {
			sendUserToast(resetKeysToast(resetKeys))
		}
		let processed: Record<string, any>
		try {
			processed = await processSecretArgs(enforced, draft.schema as any, workspace)
		} catch (e) {
			submitting = false
			sendUserToast('Failed to process sensitive args: ' + e, true)
			return
		}
		// The callback can still go away across the processSecretArgs round trip, and by
		// then the ephemeral variables exist — say so rather than leaving a dead button.
		if (!aiChatManager.handleRunFormSubmit(toolCallId, processed)) {
			submitting = false
			sendUserToast('This run form is no longer active — ask again to run the script.', true)
		}
	}
</script>

<!-- The card's heading and its chrome belong to RunScriptCard, which renders this form
     as one phase of the same card. The keyboard-scope marker says a form owns the keys
     here, so a list's own shortcuts stand down (ItemsList's SKIP_SELECTOR) — it belongs
     on this phase only, which is why a settled card drops it with the form. -->
<div
	bind:this={cardNode}
	class={twMerge('flex flex-col', layout === 'pane' ? 'h-full min-h-0' : 'pt-3')}
	data-chat-keyboard-scope="run-args-form"
>
	<!-- Only the fields scroll, sized by their content rather than filling the host: the actions
	     follow the last field of a short form, and a long one scrolls under them rather than
	     pushing the Run button — and the lines naming what the form dropped — below the fold. -->
	<div class={twMerge('relative flex flex-col', layout === 'pane' ? 'min-h-0' : '')}>
		<!-- `pl-9` is the rail the pane header's title stands on, its padding plus the icon and
		     the gap beside it, so the tab reads down one edge. The card has no such rail to meet
		     and reserves the gutter on both sides instead, which keeps its fields centred. -->
		<div
			use:fadeContainer
			onscroll={measureFades}
			class={twMerge(
				'overflow-y-auto',
				layout === 'pane' ? 'min-h-0 pl-9 pr-3' : 'max-h-[min(28rem,50vh)] px-3'
			)}
			style={layout === 'pane'
				? 'scrollbar-gutter: stable;'
				: 'scrollbar-gutter: stable both-edges;'}
		>
			<div use:fadeContent>
				{#if hasArgs}
					<!-- The one thing here that runs before Run: a `dynselect-`/`dynmultiselect-`
			argument makes DynamicInput execute that entrypoint on mount to fill its options —
			a real job on the deployed script, carrying the other args as proposed, and Cancel
			does not undo it. Everything else waits for the user; keep it that way. -->
					<SchemaForm
						bind:schema={draft.schema}
						helperScript={planMode
							? undefined
							: { source: 'deployed', path: runForm.path, runnable_kind: 'script' }}
						disabled={planMode}
						{workspace}
						prettifyHeader
						bind:isValid
						bind:args={draft.args}
					/>
				{:else}
					<p class="text-xs text-secondary">This script takes no arguments.</p>
				{/if}
			</div>
		</div>
		<!-- Only what is still below, and only while there is some: the fade the tool cards draw
		     under their own content, over the scroller rather than inside it. Nothing at the top —
		     having scrolled down is itself the knowledge that there is more up there. -->
		{#if fades.bottom}
			<div
				class={twMerge('pointer-events-none absolute inset-x-0 bottom-0 h-[min(2rem,25%)]', fadeTo)}
			></div>
		{/if}
	</div>

	<!-- One region with the actions: these lines report on the run the button below launches,
	     and separating them would read as two subjects. No padding of its own over the fields:
	     every one of them ends on the room ArgInput keeps for a validation message, and adding
	     to it would sit the buttons twice as far below the last field as the first label sits
	     from the top. A form with no fields keeps none of that room, so there it comes back. -->
	<div
		class={twMerge(
			'flex flex-col gap-2 px-3 pb-3',
			hasArgs ? '' : 'pt-3',
			layout === 'pane' ? 'pl-9' : ''
		)}
	>
		{#if runForm.droppedKeys?.length}
			<p class="text-2xs text-secondary">
				Not an input of this script, so it will not be sent:
				<span class="font-mono">{runForm.droppedKeys.join(', ')}</span>
			</p>
		{/if}
		{#if runForm.unshowableKeys?.length}
			<p class="text-2xs text-secondary">
				Sent in a shape this form has no field for, so it opened empty:
				<span class="font-mono">{runForm.unshowableKeys.join(', ')}</span>
			</p>
		{/if}
		{#if runForm.resetKeys?.length}
			<p class="text-2xs text-secondary">
				Disabled by this script, so it will run with its default:
				<span class="font-mono">{runForm.resetKeys.join(', ')}</span>
			</p>
		{/if}
		{#if runForm.strippedKeys?.length}
			<p class="text-2xs text-secondary">
				A secret or a file, so it opened empty for you to fill in:
				<span class="font-mono">{runForm.strippedKeys.join(', ')}</span>
			</p>
		{/if}
		{#if planMode}
			<p class="text-2xs text-secondary">{PLAN_MODE_MESSAGES.runFormRefused}</p>
		{/if}

		<!-- Reject then confirm at the end of the row, as ToolConfirmationFooter puts them: this
		     is a tool call being validated. Both rest while a submit is in flight — the ephemeral
		     variables exist by then, and cancelling would settle the call as declined on a run
		     already starting. Escape stops the turn from here and nowhere else in the form. -->
		<div class="flex items-center justify-end gap-2" data-run-form-actions>
			<Button
				variant="default"
				unifiedSize="sm"
				startIcon={{ icon: X }}
				disabled={submitting}
				onClick={() => aiChatManager.handleRunFormCancel(toolCallId)}
			>
				Cancel
			</Button>
			<Button
				variant="accent"
				unifiedSize="sm"
				startIcon={{ icon: Play }}
				disabled={!isValid || submitting || planMode}
				onClick={run}
			>
				Run
			</Button>
		</div>
	</div>
</div>
