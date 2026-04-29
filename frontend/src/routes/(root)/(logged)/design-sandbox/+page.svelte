<script lang="ts">
	import { onMount } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import AIChat from '$lib/components/copilot/chat/AIChat.svelte'
	import FlowWrapper from '$lib/components/FlowWrapper.svelte'
	import { chatState } from '$lib/components/copilot/chat/sharedChatState.svelte'
	import { aiChatManager, AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import ScheduleEditorInner from '$lib/components/triggers/schedules/ScheduleEditorInner.svelte'
	import {
		previewModalState,
		closePreviewModal
	} from '$lib/components/copilot/chat/previewModalState.svelte'
	import { MessageSquare } from 'lucide-svelte'

	// Fake conversation: user asks the assistant to build a 3-step user-engagement flow.
	const fakeMessages: DisplayMessage[] = [
		{
			role: 'user',
			index: 0,
			content:
				'I need a flow that pulls users from our API every morning, filters out the inactive ones, and sends a re-engagement email to the rest.'
		},
		{
			role: 'assistant',
			content:
				"Sounds good — I'll wire that up as three steps: a fetch from your `/users` endpoint, a filter on `last_active_at`, and a notify step that sends the email. Let me add them now."
		},
		{
			role: 'tool',
			tool_call_id: 'tool_step_a',
			toolName: 'create_step',
			content: 'Created step `a` — Fetch users',
			parameters: { id: 'a', summary: 'Fetch users', language: 'bun' },
			result: { ok: true, id: 'a' },
			showDetails: false
		},
		{
			role: 'tool',
			tool_call_id: 'tool_step_b',
			toolName: 'create_step',
			content: 'Created step `b` — Filter active',
			parameters: { id: 'b', summary: 'Filter active', language: 'python3' },
			result: { ok: true, id: 'b' },
			showDetails: false
		},
		{
			role: 'tool',
			tool_call_id: 'tool_step_c',
			toolName: 'create_step',
			content: 'Created step `c` — Notify',
			parameters: { id: 'c', summary: 'Notify', language: 'bun' },
			result: { ok: true, id: 'c' },
			showDetails: false
		},
		{
			role: 'assistant',
			content:
				'All three steps are in place:\n\n- **Fetch users** calls `GET /users` and returns the list.\n- **Filter active** keeps users whose `last_active_at` is within the last 30 days.\n- **Notify** sends the `re_engagement_v2` email template to each one.\n\nYou can hit **Test flow** in the top-right to dry-run it. Want me to add a daily schedule trigger as well?'
		},
		{
			role: 'user',
			index: 1,
			content:
				'Yes please, every weekday at 9am Europe/Paris. Also can the email use `name` instead of `first_name`?'
		},
		{
			role: 'assistant',
			content: "On it — adding the schedule and updating the Notify step's template variable."
		},
		{
			role: 'tool',
			tool_call_id: 'tool_schedule',
			toolName: 'add_schedule',
			content: 'Added weekday schedule at 09:00 Europe/Paris',
			parameters: { cron: '0 9 * * 1-5', timezone: 'Europe/Paris' },
			result: { ok: true, schedule_id: 'sched_001' },
			showDetails: false
		}
	]

	// Realistic-sounding past-chat titles for the sidebar history. Just titles + ids
	// because the sidebar snippet only reads those two fields.
	const fakePastChats = [
		{ id: 'mock_1', title: 'Daily user re-engagement flow' },
		{ id: 'mock_2', title: 'Sync Stripe customers to Postgres' },
		{ id: 'mock_3', title: 'Weekly usage report email' },
		{ id: 'mock_4', title: 'Migrate S3 bucket to GCS' },
		{ id: 'mock_5', title: 'Slack alert on failed deploys' },
		{ id: 'mock_6', title: 'Parse CSV uploads into Postgres' },
		{ id: 'mock_7', title: 'Generate signed download URLs' },
		{ id: 'mock_8', title: 'Onboarding webhook handler' }
	]

	function applyMocks() {
		// Pretend Windmill AI is enabled with an Anthropic model so the chat input is active.
		copilotInfo.set({
			enabled: true,
			defaultModel: { model: 'claude-sonnet-4-5', provider: 'anthropic' },
			aiModels: [{ model: 'claude-sonnet-4-5', provider: 'anthropic' }],
			customPrompts: {},
			maxTokensPerModel: {}
		})
		aiChatManager.mode = AIMode.FLOW
		aiChatManager.displayMessages = fakeMessages

		// Patch getPastChats so the sidebar's "AI chat" history shows the mock titles
		// without writing anything to IndexedDB (keeps the real workspace history clean).
		// Cast to any because savedChats / pastChats are private fields on HistoryManager.
		;(aiChatManager.historyManager as any).getPastChats = () => fakePastChats
	}

	// The parent (logged) layout always renders its AiChatLayout (right-side chat).
	// We hide it here and render our own AIChat on the left of the page.
	onMount(() => {
		chatState.size = 0
		applyMocks()
		// loadCopilot() in the parent layout is async and may overwrite our mock once
		// it resolves — re-apply after a beat to be safe for design preview.
		setTimeout(applyMocks, 1500)
	})

	// Real schedule used by the schedule preview modal. The path must exist in the
	// workspace — ScheduleEditorInner.openEdit fetches it via ScheduleService.getSchedule.
	// Synthesizing a `defaultCfg` instead leaves too many fields undefined and crashes
	// downstream sub-components (Path, ErrorOrRecoveryHandler, etc.).
	const SANDBOX_SCHEDULE_PATH = 'u/guilhempw/fake_flow_schedule'
	const SANDBOX_SCHEDULE_IS_FLOW = true

	let scheduleEditor: ScheduleEditorInner | undefined = $state(undefined)

	// When the schedule modal opens (kind = 'schedule'), load the real schedule so
	// the editor populates fully (cron, timezone, runnable, handlers, etc.).
	$effect(() => {
		if (previewModalState.kind === 'schedule' && scheduleEditor) {
			scheduleEditor.openEdit(SANDBOX_SCHEDULE_PATH, SANDBOX_SCHEDULE_IS_FLOW)
		}
	})

	// Minimal fake flow with a few modules so the editor has something to show.
	let flowStore = $state({
		val: {
			summary: 'Sandbox flow',
			description: 'A fake flow used to prototype design changes.',
			value: {
				modules: [
					{
						id: 'a',
						summary: 'Fetch users',
						value: {
							type: 'rawscript' as const,
							language: 'bun' as const,
							content:
								"export async function main() {\n  return [{ id: 1, name: 'Ada' }, { id: 2, name: 'Linus' }]\n}\n",
							input_transforms: {}
						}
					},
					{
						id: 'b',
						summary: 'Filter active',
						value: {
							type: 'rawscript' as const,
							language: 'python3' as const,
							content: 'def main(users: list):\n    return [u for u in users if u]\n',
							input_transforms: {
								users: { type: 'javascript' as const, expr: 'results.a' }
							}
						}
					},
					{
						id: 'c',
						summary: 'Notify',
						value: {
							type: 'rawscript' as const,
							language: 'bun' as const,
							content:
								"export async function main(users: any[]) {\n  console.log('notifying', users.length)\n}\n",
							input_transforms: {
								users: { type: 'javascript' as const, expr: 'results.b' }
							}
						}
					}
				]
			},
			path: 'u/admin/sandbox_flow',
			edited_at: '',
			edited_by: '',
			archived: false,
			extra_perms: {},
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {},
				required: [],
				type: 'object'
			}
		}
	})

	let flowStateStore = $state({ val: {} })

	let customUi: FlowBuilderWhitelabelCustomUi = {
		aiAgent: true
	}

	// Conversation title shown in the chat topbar. Derived from the first user
	// message in the current chat, truncated. Falls back to a placeholder while
	// the chat is empty.
	const conversationTitle = $derived.by(() => {
		const firstUser = aiChatManager.displayMessages.find((m) => m.role === 'user')
		if (!firstUser) return 'New conversation'
		const text = firstUser.content.trim()
		const truncated = text.length > 60 ? text.slice(0, 60).trimEnd() + '…' : text
		return truncated
	})
</script>

<div class="h-full w-full flex flex-col min-h-0">
	<Splitpanes horizontal={false} class="design-sandbox-splitpanes flex-1 min-h-0">
		<Pane size={50} minSize={20} class="flex flex-col min-h-0">
			<div class="flex flex-row items-center gap-2 px-3 h-12 bg-surface shrink-0">
				<MessageSquare class="w-3.5 h-3.5 text-secondary shrink-0" />
				<span class="text-xs font-medium text-primary truncate" title={conversationTitle}>
					{conversationTitle}
				</span>
			</div>
			<div class="flex-1 min-h-0">
				<AIChat />
			</div>
		</Pane>
		<!-- The right pane is a contained, padded card to make it visually
		     clear that the flow view "belongs to" the chat on the left rather
		     than being its own root view. -->
		<Pane size={50} minSize={30} class="flex flex-col min-h-0 p-2 pl-0">
			<div
				class="flex-1 min-h-0 rounded-xl border border-light bg-surface overflow-hidden shadow-sm"
			>
				<FlowWrapper
					disableAi
					pathStoreInit="u/admin/sandbox_flow"
					{customUi}
					selectedId={undefined}
					newFlow
					{flowStore}
					{flowStateStore}
				/>
			</div>
		</Pane>
	</Splitpanes>
</div>

<!-- Centered modal triggered by clicking a schedule preview card. Hosts
     ScheduleEditorInner inline (no Drawer) so the user sees the same form
     fields as on the schedule page. Two-way bind on isOpen syncs the modal's
     internal close (X / esc / outside-click) back to the shared state. -->
<Modal2
	title={`Edit schedule ${SANDBOX_SCHEDULE_PATH}`}
	fixedWidth="lg"
	fixedHeight="xl"
	contentClasses="overflow-y-auto"
	target="body"
	bind:isOpen={
		() => previewModalState.kind === 'schedule',
		(v) => {
			if (!v) closePreviewModal()
		}
	}
>
	<div class="w-full">
		<ScheduleEditorInner bind:this={scheduleEditor} useDrawer={false} />
	</div>
</Modal2>

<style>
	/* Hide the visible splitpanes divider while keeping the hit area so the
	   panel is still resizable. Scoped to this page via the wrapper class. */
	:global(.design-sandbox-splitpanes > .splitpanes__splitter) {
		background-color: transparent !important;
		background-image: none !important;
		border: none !important;
	}
	:global(.design-sandbox-splitpanes > .splitpanes__splitter::before),
	:global(.design-sandbox-splitpanes > .splitpanes__splitter::after) {
		display: none !important;
	}
</style>
