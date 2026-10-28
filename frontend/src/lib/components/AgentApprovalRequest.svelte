<script lang="ts">
	import { answerAgentRequest, type AgentRequest } from './agentRequest'
	import { Button } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import ApprovalActions from './approvals/ApprovalActions.svelte'
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
	let picked = $state<string[]>([])
	let answered = $state<boolean | undefined>(undefined)

	const choices = $derived(request.type === 'question' ? (request.choices ?? []) : [])
	const multiSelect = $derived(request.type === 'question' && !!request.multi_select)
	// A typed answer joins the picked choices when several can be picked, and replaces the pick
	// otherwise.
	const answer = $derived.by((): string | string[] => {
		const typed = text.trim()
		if (multiSelect) return typed ? [...picked, typed] : picked
		return typed || picked[0] || ''
	})

	function pick(choice: string) {
		if (multiSelect) {
			picked = picked.includes(choice) ? picked.filter((c) => c !== choice) : [...picked, choice]
		} else {
			picked = [choice]
			text = ''
		}
	}

	async function onAnswer(approved: boolean) {
		const value = request.type === 'question' ? answer : text.trim()
		await answerAgentRequest(workspaceId, { agentJobId, action: request }, approved, value)
		answered = approved
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
				{answered ? 'Answered' : 'Declined'}, the agent resumes shortly.
			{:else}
				{answered ? 'Approved' : 'Rejected'}, the agent resumes shortly.
			{/if}
		</div>
	{:else if pending}
		<div class="text-hint">Expires {displayDate(request.expires_at)}</div>
		{#if choices.length > 0}
			<div class="flex flex-wrap gap-2">
				{#each choices as choice (choice)}
					<Button
						variant="default"
						unifiedSize="sm"
						selected={picked.includes(choice)}
						onClick={() => pick(choice)}
					>
						{choice}
					</Button>
				{/each}
			</div>
		{/if}
		<TextInput
			bind:value={text}
			inputProps={{
				placeholder:
					request.type === 'question'
						? choices.length > 0
							? 'Or answer in your own words'
							: 'Your answer'
						: 'Reason for the agent, if you reject (optional)',
				oninput: () => {
					if (!multiSelect) picked = []
				}
			}}
			size="sm"
		/>
		{#if request.type === 'question'}
			<ApprovalActions
				approveLabel="Send answer"
				rejectTitle="Decline to answer"
				approveDisabled={answer.length === 0}
				{onAnswer}
			/>
		{:else}
			<ApprovalActions approveLabel="Approve" rejectTitle="Reject" {onAnswer} />
		{/if}
	{/if}
</div>
