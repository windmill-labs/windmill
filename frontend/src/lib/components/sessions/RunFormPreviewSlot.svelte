<script lang="ts">
	import { setContext, untrack } from 'svelte'
	import { Code } from 'lucide-svelte'
	import RunArgsFormDisplay from '$lib/components/copilot/chat/RunArgsFormDisplay.svelte'
	import { isActiveRunForm, type ToolDisplayMessage } from '$lib/components/copilot/chat/shared'
	import type { AIChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'

	interface Props {
		manager: AIChatManager
		toolCallId: string
	}

	let { manager, toolCallId }: Props = $props()

	// The panel is not under the chat's context (SessionEditorTarget sets it for the same
	// reason), and RunArgsFormDisplay resolves its manager from it. Without this the form
	// binds to the app-wide singleton: a different draft, and a plan-mode flag that is not
	// this session's — which renders every field disabled.
	// Captured at init, as SessionEditorTarget does: a reused instance keeps the first
	// runtime's manager, and descendants rely on the context's presence, not its identity.
	setContext(
		'aiChatManager',
		untrack(() => manager)
	)

	// The card's own message, so the pane renders the same form the chat holds rather than a
	// second one: RunArgsFormDisplay resolves the loop's pending callback through the manager,
	// which is why pressing Run here is the chat pressing Run.
	const message = $derived(
		manager.displayMessages.find(
			(m): m is ToolDisplayMessage =>
				m.role === 'tool' && m.tool_call_id === toolCallId && !!m.runForm
		)
	)
	// A tab outlives a chat rotation, and a settled form has nothing left to fill in.
	const pending = $derived(message ? isActiveRunForm(message) : false)
</script>

<!-- surface-tertiary because the form's scroll fades gradient from it: the card is that
     colour, and a different ground here would leave a visible band at each fade. -->
<div class="flex h-full min-h-0 flex-col bg-surface-tertiary">
	{#if message?.runForm && pending}
		<!-- No rule under it, as ArtifactViewer's header has none: the fields scroll under a
		     fade, and a border would draw that same boundary a second time. -->
		<div class="flex items-center gap-2 p-3">
			<Code class="h-4 w-4 shrink-0 text-accent" />
			<div class="min-w-0 flex-1">
				<p class="truncate text-xs font-semibold text-emphasis">
					Run {message.runForm.summary || message.runForm.path}
				</p>
				{#if message.runForm.summary}
					<p class="truncate font-mono text-2xs text-secondary">{message.runForm.path}</p>
				{/if}
			</div>
		</div>
		<RunArgsFormDisplay {toolCallId} runForm={message.runForm} layout="pane" />
	{:else}
		<div class="p-4 text-sm text-tertiary">This run form is no longer available.</div>
	{/if}
</div>
