<script lang="ts">
	import { answerAgentRequest, type AgentRequest } from './agentRequest'
	import { sendUserToast } from '$lib/toast'
	import { Button } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import { displayDate } from '$lib/utils'

	interface Props {
		/** The workspace the job ran in, never the navigation workspace. */
		workspaceId: string
		/** The agent step's own job: it is the one parked, so it is the one answered. */
		agentJobId: string
		/** Its call id goes with the answer, so a page left open on an earlier request cannot
		 *  answer the one the agent waits on now. */
		request: AgentRequest
		/** Whether the request still waits for an answer. */
		pending: boolean
	}

	let { workspaceId, agentJobId, request, pending }: Props = $props()

	let text = $state('')
	let loading = $state(false)
	let answered = $state<boolean | undefined>(undefined)

	async function answer(approved: boolean) {
		loading = true
		try {
			await answerAgentRequest(workspaceId, { agentJobId, action: request }, approved, text.trim())
			answered = approved
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed to answer the request', true)
		} finally {
			loading = false
		}
	}
</script>

<div class="flex flex-col gap-2 w-full text-xs">
	{#if request.type === 'question'}
		<div class="text-emphasis font-semibold">The agent {pending ? 'asks' : 'asked'} a question</div>
		<div class="text-primary whitespace-pre-wrap">{request.question}</div>
	{:else}
		<div class="text-emphasis font-semibold">
			The agent {pending ? 'asks' : 'asked'} to run {request.calls.length === 1
				? 'this call'
				: 'these calls'}
		</div>
		{#each request.calls as call, i (i)}
			<div class="flex flex-col gap-1">
				<span class="font-mono text-primary">{call.function_name}</span>
				<pre
					class="text-2xs bg-surface-secondary rounded-md p-2 overflow-auto max-h-40 whitespace-pre-wrap break-all"
					>{JSON.stringify(call.arguments, null, 2)}</pre
				>
			</div>
		{/each}
	{/if}
	{#if answered !== undefined}
		<div class="text-secondary">
			{#if request.type === 'question'}
				Answered, the agent resumes shortly.
			{:else}
				{answered ? 'Approved' : 'Rejected'}, the agent resumes shortly.
			{/if}
		</div>
	{:else if pending}
		<div class="text-hint">Expires {displayDate(request.expires_at)}</div>
		{#if request.type === 'question'}
			<TextInput
				bind:value={text}
				inputProps={{
					placeholder: 'Your answer',
					disabled: loading,
					onkeydown: (e) => {
						if (e.key === 'Enter' && !loading && text.trim()) answer(true)
					}
				}}
				size="sm"
			/>
			<div class="flex justify-end">
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={loading || !text.trim()}
					onClick={() => answer(true)}
				>
					Send answer
				</Button>
			</div>
		{:else}
			<TextInput
				bind:value={text}
				inputProps={{
					placeholder: 'Reason for the agent, if you reject (optional)',
					disabled: loading
				}}
				size="sm"
			/>
			<div class="flex gap-2 justify-end">
				<Button
					variant="default"
					destructive
					unifiedSize="sm"
					disabled={loading}
					onClick={() => answer(false)}
				>
					Reject
				</Button>
				<Button variant="accent" unifiedSize="sm" disabled={loading} onClick={() => answer(true)}>
					Approve
				</Button>
			</div>
		{/if}
	{/if}
</div>
