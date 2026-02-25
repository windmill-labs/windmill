<script lang="ts" module>
	export type OnBehalfOfChoice = 'source' | 'target' | 'me' | undefined

	/**
	 * Check if an item needs on_behalf_of selection (more than 1 unique option available)
	 */
	export function needsOnBehalfOfSelection(
		kind: string,
		sourceEmail: string | undefined,
		targetEmail: string | undefined,
		myEmail: string | undefined
	): boolean {
		if (kind !== 'flow' && kind !== 'script' && kind !== 'app' && kind !== 'trigger') return false

		// Don't show if no on_behalf_of is set in source
		if (!sourceEmail) return false

		// Count unique options: source, target (even if undefined counts as different), me
		const options = new Set([sourceEmail, myEmail])
		// Target is a unique option if it differs from source (including undefined != defined)
		if (targetEmail !== sourceEmail) {
			options.add(targetEmail ?? '__not_set__')
		}

		// Show if more than 1 unique option
		return options.size > 1
	}
</script>

<script lang="ts">
	import { Check, UserCog } from 'lucide-svelte'
	import MeltPopover from './meltComponents/Popover.svelte'
	import { userStore } from '$lib/stores'

	interface Props {
		sourceEmail: string | undefined
		targetEmail: string | undefined
		selected: OnBehalfOfChoice
		onSelect: (choice: OnBehalfOfChoice) => void
		kind: string
		canPreserve: boolean
	}

	let { sourceEmail, targetEmail, selected, onSelect, kind, canPreserve }: Props = $props()

	let label = $derived(
		kind === 'trigger'
			? 'Set the user this will be recorded as edited by:'
			: 'Set the user this will be run on behalf of:'
	)
</script>

<MeltPopover placement="bottom">
	<svelte:fragment slot="trigger">
		<UserCog class="w-4 h-4 {selected ? 'text-green-500' : 'text-yellow-500'}" />
	</svelte:fragment>
	<div slot="content" class="p-3 flex flex-col gap-2 min-w-48">
		<div class="text-xs font-medium text-secondary mb-1">{label}</div>
		<button
			class="flex items-center gap-2 px-2 py-1.5 rounded text-left text-xs hover:bg-surface-hover {!canPreserve
				? 'opacity-50 cursor-not-allowed'
				: ''}"
			disabled={!canPreserve}
			onclick={() => onSelect('source')}
		>
			<Check class="w-3 h-3 {selected === 'source' ? 'opacity-100' : 'opacity-0'}" />
			<span class="truncate max-w-40">{sourceEmail}</span>
			<span class="text-xs text-tertiary">(source)</span>
		</button>
		<button
			class="flex items-center gap-2 px-2 py-1.5 rounded text-left text-xs hover:bg-surface-hover {!canPreserve || !targetEmail
				? 'opacity-50 cursor-not-allowed'
				: ''}"
			disabled={!canPreserve || !targetEmail}
			onclick={() => onSelect('target')}
		>
			<Check class="w-3 h-3 {selected === 'target' ? 'opacity-100' : 'opacity-0'}" />
			<span class="truncate max-w-40 {!targetEmail ? 'italic text-tertiary' : ''}"
				>{targetEmail ?? 'unknown'}</span
			>
			<span class="text-xs text-tertiary">(target)</span>
		</button>
		<button
			class="flex items-center gap-2 px-2 py-1.5 rounded text-left text-xs hover:bg-surface-hover"
			onclick={() => onSelect('me')}
		>
			<Check class="w-3 h-3 {selected === 'me' ? 'opacity-100' : 'opacity-0'}" />
			<span class="truncate max-w-40"
				>{kind === 'trigger' ? $userStore?.username : $userStore?.email}</span
			>
			<span class="text-xs text-tertiary">(me)</span>
		</button>
	</div>
</MeltPopover>
