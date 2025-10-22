<script lang="ts">
	import Badge from '../common/badge/Badge.svelte'
	import { getTriggerLabel, type Trigger } from './utils'
	import { twMerge } from 'tailwind-merge'

	let { trigger, discard }: { trigger: Trigger; discard?: boolean } = $props()

	let label = $derived.by(() => getTriggerLabel(trigger))
</script>

<span
	class={twMerge(
		trigger.isDraft ? 'text-hint' : 'font-normal',
		'truncate pr-1',
		discard ? 'line-through' : ''
	)}
	title={label}
>
	{label}
</span>

{#if trigger.type === 'default_email'}
	<span
		class="ml-2 bg-blue-50 dark:bg-blue-900/40 px-1.5 py-0.5 rounded text-xs text-blue-700 dark:text-blue-100 whitespace-nowrap"
	>
		Default
	</span>
{/if}

{#if trigger.isPrimary}
	<Badge color="blue">Primary</Badge>
{/if}

{#if trigger.draftConfig && !trigger.isDraft}
	<Badge color="orange">Modified</Badge>
{/if}

{#if trigger.isDraft}
	<Badge color="gray">New</Badge>
{/if}
