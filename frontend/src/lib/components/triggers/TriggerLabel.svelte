<script lang="ts">
	import Badge from '../common/badge/Badge.svelte'
	import { getTriggerLabel, type Trigger } from './utils'
	import { twMerge } from 'tailwind-merge'

	let { trigger, discard }: { trigger: Trigger; discard?: boolean } = $props()

	let label = $derived.by(() => getTriggerLabel(trigger))
</script>

<span class={twMerge('truncate pr-2', discard ? 'line-through' : '')} title={label}>
	{label}
</span>

{#if trigger.type === 'default_email' || trigger.type === 'webhook' || trigger.type === 'cli'}
	<Badge color="gray">Default</Badge>
{/if}

{#if trigger.isPrimary}
	<Badge color="blue">Primary</Badge>
{/if}

{#if trigger.draftConfig && !trigger.isDraft}
	<Badge color="orange">Modified</Badge>
{/if}

{#if trigger.isDraft}
	<Badge color="indigo">Draft</Badge>
{/if}
