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
		},
		{
			role: 'tool',
			tool_call_id: 'tool_edit_c',
			toolName: 'edit_step',
			content: 'Updated `Notify` to reference `user.name`',
			parameters: { id: 'c', diff: '- {{ user.first_name }}\n+ {{ user.name }}' },
			result: { ok: true },
			showDetails: false,
			isLoading: true
		}
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
</script>

<div class="h-full w-full flex flex-col min-h-0">
	<Splitpanes horizontal={false} class="flex-1 min-h-0">
		<Pane size={50} minSize={20} class="flex flex-col min-h-0 border-r border-light">
			<AIChat />
		</Pane>
		<Pane size={50} minSize={30} class="flex flex-col min-h-0">
			<FlowWrapper
				disableAi
				pathStoreInit="u/admin/sandbox_flow"
				{customUi}
				selectedId={undefined}
				newFlow
				{flowStore}
				{flowStateStore}
			/>
		</Pane>
	</Splitpanes>
</div>
