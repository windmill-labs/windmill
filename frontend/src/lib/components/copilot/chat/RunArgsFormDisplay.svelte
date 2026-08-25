<script lang="ts">
	import { onMount, tick } from 'svelte'
	import { Play, X } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { processSecretArgs } from '$lib/components/secretArgUtils'
	import { conformArgsToSchema } from '$lib/components/job_args'
	import { sendUserToast } from '$lib/utils'
	import { getAiChatManager } from './aiChatManagerContext'
	import type { RunFormDisplay } from './shared'

	// Never the imported singleton: submitting has to resolve the pending callback of
	// the manager that opened this form, which in a session is a per-pane one.
	const aiChatManager = getAiChatManager()

	interface Props {
		toolCallId: string
		runForm: RunFormDisplay
	}

	let { toolCallId, runForm }: Props = $props()

	// The chat's workspace, not the globally-active one: a session may be acting on a
	// fork, and that is where the job runs — so the pickers and the ephemeral secret
	// variables have to resolve there too.
	const workspace = $derived(aiChatManager.operatingWorkspace)

	const properties = $derived(runForm.schema?.properties ?? {})
	const hasArgs = $derived(Object.keys(properties).length > 0)

	// Deep copy, not a spread: runForm comes off displayMessages ($state), so its nested
	// values are proxies that $state() hands back untouched. SchemaForm edits objects and
	// arrays in place, so a shallow copy would write every keystroke — a password typed
	// into a nested field included — straight into the persisted transcript.
	let args = $state($state.snapshot(runForm.args ?? {}) as Record<string, any>)
	let isValid = $state(true)
	let submitting = $state(false)
	let cardNode = $state<HTMLDivElement | undefined>()

	onMount(() => {
		void tick().then(() => cardNode?.scrollIntoView({ block: 'nearest' }))
	})

	async function run() {
		if (submitting || !isValid) return
		// Before processSecretArgs, not after: a card restored from history outlives the
		// manager that opened it, so submitting would mint an ephemeral secret variable
		// per click and still run nothing.
		if (!aiChatManager.isRunFormPending(toolCallId)) {
			sendUserToast('This run form is no longer active — ask again to run the script.', true)
			return
		}
		submitting = true
		let processed: Record<string, any>
		try {
			processed = await processSecretArgs(
				// Last gate before the job: what the card showed is what runs, conformed the
				// same way the prefill was.
				conformArgsToSchema(args ?? {}, runForm.schema).args,
				runForm.schema as any,
				workspace
			)
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

<!-- scroll-mb clears the chat's sticky "Waiting for your input" chip so the mount
     scrollIntoView leaves the Run button uncovered. -->
<div
	bind:this={cardNode}
	class="scroll-mb-8 rounded-md border border-border-light bg-surface p-3"
	data-chat-keyboard-scope="run-args-form"
>
	<div class="flex items-start gap-2">
		<Play class="h-4 w-4 shrink-0 text-accent" />
		<div class="min-w-0 flex-1">
			<p class="truncate text-xs font-semibold text-emphasis">
				Run {runForm.summary || runForm.path}
			</p>
			{#if runForm.summary}
				<p class="truncate font-mono text-2xs text-secondary">{runForm.path}</p>
			{/if}
		</div>
	</div>

	<div class="mt-3">
		{#if hasArgs}
			<SchemaForm
				schema={runForm.schema}
				helperScript={{ source: 'deployed', path: runForm.path, runnable_kind: 'script' }}
				{workspace}
				prettifyHeader
				lightHeader
				bind:isValid
				bind:args
			/>
		{:else}
			<p class="text-xs text-secondary">This script takes no arguments.</p>
		{/if}
		{#if runForm.droppedKeys?.length}
			<p class="mt-2 text-2xs text-secondary">
				Not an input of this script, so it will not be sent:
				<span class="font-mono">{runForm.droppedKeys.join(', ')}</span>
			</p>
		{/if}
		{#if runForm.resetKeys?.length}
			<p class="mt-2 text-2xs text-secondary">
				Disabled by this script, so it will run with its default:
				<span class="font-mono">{runForm.resetKeys.join(', ')}</span>
			</p>
		{/if}
	</div>

	<!-- Both buttons rest while a submit is in flight: the ephemeral variables exist by
	     then, so cancelling would settle the call as declined on a run that is already
	     starting. Marked as the one part of the form Escape still stops the turn from. -->
	<div class="mt-3 flex items-center gap-2" data-run-form-actions>
		<Button
			variant="accent"
			unifiedSize="sm"
			startIcon={{ icon: Play }}
			disabled={!isValid || submitting}
			onClick={run}
		>
			Run
		</Button>
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: X }}
			disabled={submitting}
			onClick={() => aiChatManager.handleRunFormCancel(toolCallId)}
		>
			Cancel
		</Button>
	</div>
</div>
