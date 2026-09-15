<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { keepsManagedMemory } from '../agentFormFields'

	interface Props {
		/** The agent's input transforms. Converting a legacy setting writes `memory` and the step input
		 *  it moves into. */
		args: Record<string, any>
		chatInputEnabled?: boolean
		/** Whether the step's own memory id and messages are on this form. A saved agent has neither:
		 *  they belong to each step linking it. */
		historyOnStep?: boolean
		s3StorageConfigured?: boolean
	}

	let {
		args = $bindable(),
		chatInputEnabled = false,
		historyOnStep = false,
		s3StorageConfigured = true
	}: Props = $props()

	let memory = $derived(
		args?.memory?.type === 'static'
			? (args.memory.value as Record<string, any> | null | undefined)
			: undefined
	)
	let on = $derived(keepsManagedMemory(memory))
	// `auto` and `manual` are what older editors wrote. They stay as they are until the author
	// converts them, so an untouched step still runs on an older worker.
	let legacyMessages = $derived(
		memory?.kind === 'manual' ? ((memory.messages ?? []) as unknown[]) : undefined
	)
	// A chat run always carries the conversation's memory id, so there a baked id was never read.
	let legacyMemoryId = $derived(
		on && memory?.kind === 'auto' && memory.memory_id && !chatInputEnabled
			? String(memory.memory_id)
			: undefined
	)

	// An `auto` setting whose saved id is never read runs exactly like the current setting for its
	// state, so switching to that setting is the only choice.
	let legacyEquivalent = $derived(memory?.kind === 'auto' && !legacyMemoryId)

	function switchToEquivalent() {
		args.memory = {
			type: 'static',
			value: on ? { kind: 'window', context_length: memory?.context_length } : { kind: 'off' }
		}
	}

	function convertLegacyMemoryId(keepAsMemoryId: boolean) {
		if (keepAsMemoryId && legacyMemoryId) {
			args.memory_id = { type: 'static', value: legacyMemoryId }
		}
		args.memory = {
			type: 'static',
			value: { kind: 'window', context_length: memory?.context_length }
		}
	}

	function moveMessagesToStep() {
		args.messages = { type: 'static', value: $state.snapshot(legacyMessages) ?? [] }
		args.memory = { type: 'static', value: { kind: 'off' } }
	}
</script>

{#if on && !s3StorageConfigured}
	<p class="mt-1 text-2xs text-hint">
		Without S3 storage on the workspace, memory is kept in the database, up to 100KB per memory.
	</p>
{/if}
{#if legacyMessages}
	<Alert type="info" title="Older memory setting" class="mt-2">
		<div class="flex flex-col gap-2">
			<span>
				An earlier version of the editor saved a fixed list of messages here, which this agent still
				sends.
			</span>
			{#if historyOnStep}
				<div class="flex">
					<Button
						variant="default"
						unifiedSize="sm"
						btnClasses="bg-surface"
						onclick={moveMessagesToStep}
					>
						Move to previous messages
					</Button>
				</div>
			{/if}
		</div>
	</Alert>
{/if}
{#if legacyMemoryId}
	<Alert type="info" title="Older memory setting" class="mt-2">
		<div class="flex flex-col gap-2">
			<span>
				{historyOnStep
					? 'Fixed memory id generated when this flow was saved.'
					: 'Fixed memory id saved with this agent.'}
				Every run shares it unless the caller passes one.
			</span>
			<div class="flex gap-2">
				{#if historyOnStep}
					<Button
						variant="default"
						unifiedSize="sm"
						btnClasses="bg-surface"
						onclick={() => convertLegacyMemoryId(true)}
					>
						Keep as memory id
					</Button>
				{/if}
				<Button
					variant="default"
					unifiedSize="sm"
					btnClasses="bg-surface"
					onclick={() => convertLegacyMemoryId(false)}
				>
					Use the run's memory id
				</Button>
			</div>
		</div>
	</Alert>
{/if}
{#if legacyEquivalent}
	<Alert type="info" title="Older memory setting" class="mt-2">
		<div class="flex flex-col gap-2">
			<span>
				An earlier version of the editor saved this setting. It works the same as {on
					? 'On'
					: 'Off'}.
			</span>
			<div class="flex">
				<Button
					variant="default"
					unifiedSize="sm"
					btnClasses="bg-surface"
					onclick={switchToEquivalent}
				>
					Switch to {on ? 'On' : 'Off'}
				</Button>
			</div>
		</div>
	</Alert>
{/if}
