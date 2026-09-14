<script lang="ts">
	import type { Snippet } from 'svelte'
	import { Badge, Button } from '$lib/components/common'
	import FieldHeader from '$lib/components/FieldHeader.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import type { AgentHistoryKey } from '../agentFormFields'

	type Source = 'run' | 'here' | 'messages'

	interface Props {
		/** The step's input transforms. `memory_id` and `messages` are written only by this row. */
		args: Record<string, any>
		label: string
		tooltip?: string
		chatInputEnabled?: boolean
		/** Why no memory id applies to this step, when the agent reads no memory. */
		memoryUnusedNote?: string
		/** The step's editor for one history input, with the error to show under it. */
		field: Snippet<[AgentHistoryKey, string | undefined]>
		/** Called for each key this row removes, so the form forgets its validity. */
		onRemoveKey?: (key: AgentHistoryKey) => void
	}

	let {
		args = $bindable(),
		label,
		tooltip = undefined,
		chatInputEnabled = false,
		memoryUnusedNote = undefined,
		field,
		onRemoveKey = undefined
	}: Props = $props()

	// Neither key gets a placeholder, so whichever one is present is the chosen source.
	let source: Source = $derived(args?.messages ? 'messages' : args?.memory_id ? 'here' : 'run')
	let memoryId = $derived(args?.memory_id)
	let fixedMemoryId = $derived(memoryId?.type === 'static')
	let emptyMemoryId = $derived(fixedMemoryId && !String(memoryId?.value ?? '').trim())

	function remove(key: AgentHistoryKey) {
		if (args && key in args) {
			delete args[key]
			onRemoveKey?.(key)
		}
	}

	function selectSource(next: Source) {
		if (next === source) return
		remove('memory_id')
		remove('messages')
		if (next === 'here') args.memory_id = { type: 'static', value: '' }
		if (next === 'messages') args.messages = { type: 'static', value: [] }
	}
</script>

<div class="flex w-full flex-col gap-2">
	<div class="flex min-h-7 items-end">
		<FieldHeader {label} simpleTooltip={tooltip} displayType={false} />
	</div>
	{#if memoryUnusedNote}
		<p class="text-xs text-secondary">{memoryUnusedNote}</p>
		{#if source === 'here'}
			<div class="flex items-center gap-2">
				<Badge color="yellow" small>Ignored</Badge>
				<Button variant="subtle" unifiedSize="sm" onclick={() => remove('memory_id')}>
					Clear memory id
				</Button>
			</div>
			{@render field('memory_id', undefined)}
		{/if}
		{#if source === 'messages'}
			{@render field('messages', undefined)}
			<div>
				<Button variant="subtle" unifiedSize="sm" onclick={() => remove('messages')}>
					Remove messages
				</Button>
			</div>
		{:else}
			<div>
				<Button variant="default" unifiedSize="sm" onclick={() => selectSource('messages')}>
					Provide messages
				</Button>
			</div>
		{/if}
	{:else}
		<ToggleButtonGroup selected={source} onSelected={(next) => selectSource(next)}>
			{#snippet children({ item })}
				<ToggleButton value="run" label="From the run" {item} />
				<ToggleButton value="here" label="Set here" {item} />
				<ToggleButton value="messages" label="Provided messages" {item} />
			{/snippet}
		</ToggleButtonGroup>
		{#if source === 'run'}
			{#if chatInputEnabled}
				<div><Badge color="gray">Chat conversation</Badge></div>
			{:else}
				<p class="text-xs text-secondary">
					Passed by the caller as <code>memory_id</code>, stateless otherwise.
				</p>
			{/if}
		{:else if source === 'here'}
			{@render field(
				'memory_id',
				emptyMemoryId ? 'Enter a memory id, or choose From the run.' : undefined
			)}
			<p class="text-2xs text-hint">
				{#if emptyMemoryId}
					A fixed id such as <code>support-triage</code> keeps one memory shared by every run, an
					expression such as <code>flow_input.customer_id</code> one memory per key.
				{:else if fixedMemoryId}
					Every run of this step shares this memory. Overrides the memory id passed by the caller.
				{:else}
					Overrides the memory id passed by the caller.
				{/if}
			</p>
		{:else}
			{@render field('messages', undefined)}
			<p class="text-2xs text-hint">
				Messages sent before the user message, supplied by this flow instead of read from memory.
			</p>
		{/if}
	{/if}
</div>
