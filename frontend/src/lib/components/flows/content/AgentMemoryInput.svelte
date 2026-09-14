<script lang="ts">
	import type { Snippet } from 'svelte'
	import { Button } from '$lib/components/common'
	import FieldHeader from '$lib/components/FieldHeader.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { DEFAULT_AGENT_MEMORY, memoryPolicyIsOff } from '../agentFormFields'

	interface Props {
		/** The agent's input transforms. `memory` is written whole; converting a legacy setting also
		 *  writes the history input it moves into. */
		args: Record<string, any>
		label: string
		tooltip?: string
		labelExtra?: Snippet
		chatInputEnabled?: boolean
		/** Whether the step's history row is on this form, so a legacy memory id or messages can move
		 *  into it. A saved agent has none: its history belongs to each step linking it. */
		historyOnStep?: boolean
		s3StorageConfigured?: boolean
	}

	let {
		args = $bindable(),
		label,
		tooltip = undefined,
		labelExtra = undefined,
		chatInputEnabled = false,
		historyOnStep = false,
		s3StorageConfigured = true
	}: Props = $props()

	const POLICY_ITEMS = [
		{ label: 'Keep last messages', value: 'window' },
		{ label: 'Off', value: 'off' }
	]

	let memory = $derived(args?.memory?.value as Record<string, any> | null | undefined)
	// `auto` and `manual` are what older editors wrote. They are shown as they are and only rewritten
	// once the author changes the setting, so an untouched step still runs on an older worker.
	let kind = $derived(
		memory?.kind === 'manual' ? 'manual' : memoryPolicyIsOff(memory) ? 'off' : 'window'
	)
	// A chat run always carries the conversation's memory id, so there a baked id was never read.
	let legacyMemoryId = $derived(
		kind === 'window' && memory?.kind === 'auto' && memory.memory_id && !chatInputEnabled
			? String(memory.memory_id)
			: undefined
	)

	function write(value: Record<string, any>) {
		args.memory = { type: 'static', value }
	}

	function setKind(next: string) {
		if (next === kind) return
		write(next === 'window' ? structuredClone(DEFAULT_AGENT_MEMORY) : { kind: 'off' })
	}

	function setContextLength(next: string | number | undefined) {
		const contextLength = Math.floor(Number(next))
		// An emptied box would otherwise turn memory off and hide the box being typed into.
		if (!Number.isFinite(contextLength) || contextLength < 1) return
		// A baked id survives a new count: it goes only through the choice offered for it below.
		write(
			memory?.memory_id
				? { ...memory, context_length: contextLength }
				: { kind: 'window', context_length: contextLength }
		)
	}

	function convertLegacyMemoryId(keepAsOverride: boolean) {
		if (keepAsOverride && legacyMemoryId) {
			args.memory_id = { type: 'static', value: legacyMemoryId }
		}
		write({ kind: 'window', context_length: memory?.context_length })
	}

	function moveMessagesToHistory() {
		args.messages = { type: 'static', value: memory?.messages ?? [] }
		write({ kind: 'off' })
	}
</script>

<div class="flex w-full flex-col gap-1">
	<div class="flex min-h-7 items-end">
		<FieldHeader {label} simpleTooltip={tooltip} displayType={false} />
		{@render labelExtra?.()}
	</div>
	{#if kind === 'manual'}
		<div class="flex flex-col gap-2 rounded-md border px-3 py-2">
			<p class="text-xs text-secondary">
				This agent sends a fixed list of messages, set in its memory by an earlier version of the
				editor.
			</p>
			{#if historyOnStep}
				<div>
					<Button variant="default" unifiedSize="sm" onclick={moveMessagesToHistory}>
						Move to history
					</Button>
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex items-center gap-2">
			<div class="w-52">
				<Select items={POLICY_ITEMS} bind:value={() => kind, (next) => next && setKind(next)} />
			</div>
			{#if kind === 'window'}
				<div class="w-20">
					<TextInput
						inputProps={{ type: 'number', min: 1, 'aria-label': 'Messages to keep' }}
						bind:value={() => memory?.context_length, setContextLength}
					/>
				</div>
				<span class="text-xs text-secondary">messages</span>
			{/if}
		</div>
		{#if kind === 'window'}
			<p class="text-2xs text-hint">
				History is kept per memory id: the chat conversation, an app chat session, a memory id
				passed when the run starts, or one set on the step. Without a memory id the agent runs
				stateless.
			</p>
			{#if !s3StorageConfigured}
				<p class="text-2xs text-hint">
					Without S3 storage on the workspace, memory is kept in the database, up to 100KB per
					memory.
				</p>
			{/if}
		{/if}
		{#if legacyMemoryId}
			<div class="flex flex-col gap-2 rounded-md border px-3 py-2">
				<p class="text-xs text-secondary">
					{historyOnStep
						? 'Fixed memory id generated when this flow was saved.'
						: 'Fixed memory id saved with this agent.'}
					Every run shares it unless the caller passes one.
				</p>
				<div class="flex gap-2">
					{#if historyOnStep}
						<Button variant="default" unifiedSize="sm" onclick={() => convertLegacyMemoryId(true)}>
							Keep as override
						</Button>
					{/if}
					<Button variant="default" unifiedSize="sm" onclick={() => convertLegacyMemoryId(false)}>
						Use the run's memory id
					</Button>
				</div>
			</div>
		{/if}
	{/if}
</div>
