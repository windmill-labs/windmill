<script lang="ts">
	import { ChevronDown, ChevronRight, Minus, Pencil, Plus } from 'lucide-svelte'
	import Badge from '../common/badge/Badge.svelte'
	import Button from '../common/button/Button.svelte'
	import ObjectViewer from '../propertyPicker/ObjectViewer.svelte'
	import { formatScalar, isComplex, parseChanges, type FieldChange } from './triggerHistoryChanges'

	interface Props {
		changes: unknown
	}

	let { changes }: Props = $props()

	const parsed = $derived(parseChanges(changes))

	let expanded: Record<string, boolean> = $state({})

	const marker = {
		added: { icon: Plus, class: 'text-green-500' },
		removed: { icon: Minus, class: 'text-red-500' },
		changed: { icon: Pencil, class: 'text-yellow-500' }
	} as const

	/** A row needs the tree viewer when either side is an object or an array. */
	function needsTree(change: FieldChange): boolean {
		return (
			('next' in change && isComplex(change.next)) || ('prev' in change && isComplex(change.prev))
		)
	}
</script>

{#if parsed.kind === 'truncated'}
	<div class="flex flex-col gap-1.5">
		<span class="text-xs text-secondary">
			{parsed.fields.length} fields changed, too large to store in full
		</span>
		<div class="flex flex-row flex-wrap gap-1">
			{#each parsed.fields as field (field)}
				<Badge color="gray" small>{field}</Badge>
			{/each}
		</div>
	</div>
{:else if parsed.kind === 'fields'}
	<!-- One grid for the whole entry, so field names and values line up in
	     columns however many rows it has. -->
	<div class="grid grid-cols-[auto_max-content_minmax(0,1fr)] gap-x-2 items-baseline">
		{#each parsed.changes as change (change.field)}
			{@const Icon = marker[change.kind].icon}
			{@const tree = needsTree(change)}
			<Icon size={12} class={`${marker[change.kind].class} shrink-0 translate-y-0.5`} />
			<span class="text-2xs font-mono text-emphasis py-0.5">{change.field}</span>
			{#if tree}
				<Button
					unifiedSize="2xs"
					variant="subtle"
					wrapperClasses="w-fit"
					btnClasses="!text-2xs !font-normal !px-1"
					startIcon={{ icon: expanded[change.field] ? ChevronDown : ChevronRight }}
					on:click={() => (expanded[change.field] = !expanded[change.field])}
				>
					{change.kind === 'changed' ? 'value changed' : 'value'}
				</Button>
			{:else}
				<span class="text-2xs font-mono py-0.5 break-all">
					{#if change.kind === 'changed' || (change.kind === 'removed' && change.prev !== undefined)}
						<span class="text-tertiary line-through">{formatScalar(change.prev)}</span>
						<span class="text-tertiary px-1">→</span>
					{/if}
					{#if change.kind === 'removed'}
						<span class="text-tertiary italic">unset</span>
					{:else}
						<span class="text-emphasis">{formatScalar(change.next)}</span>
					{/if}
				</span>
			{/if}
			{#if tree && expanded[change.field]}
				<div class="col-span-3 pl-6 pb-1 flex flex-col gap-1 min-w-0">
					{#if change.kind !== 'added' && change.prev !== undefined}
						<div class="text-2xs text-tertiary">before</div>
						<ObjectViewer pureViewer json={change.prev} />
					{/if}
					{#if change.kind !== 'removed'}
						<div class="text-2xs text-tertiary">after</div>
						<ObjectViewer pureViewer json={change.next} />
					{/if}
				</div>
			{/if}
		{/each}
	</div>
{/if}
