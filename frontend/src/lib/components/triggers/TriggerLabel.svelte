<script lang="ts">
	import { getTriggerLabel, type Trigger } from './utils'
	import { twMerge } from 'tailwind-merge'

	let { trigger }: { trigger: Trigger } = $props()

	let label = $derived.by(() => getTriggerLabel(trigger))
</script>

<span
	class={twMerge(
		trigger.isDraft ? 'text-frost-400 italic dark:text-frost-200' : 'font-normal',
		'truncate pr-1'
	)}
	title={label}
>
	{label}
</span>

{#if trigger.isPrimary}
	<span
		class="ml-2 bg-blue-50 dark:bg-blue-900/40 px-1.5 py-0.5 rounded text-xs text-blue-700 dark:text-blue-100 whitespace-nowrap"
	>
		Primary
	</span>
{/if}

{#if trigger.draftConfig && !trigger.isDraft}
	<span
		class="ml-2 text-xs bg-amber-100 dark:bg-amber-900 text-amber-800 dark:text-amber-100 px-1.5 py-0.5 rounded whitespace-nowrap"
	>
		Modified
	</span>
{/if}

{#if trigger.isDraft}
	<span
		class="ml-2 text-xs bg-frost-400 dark:bg-frost-900 text-white px-1.5 py-0.5 rounded whitespace-nowrap"
	>
		New
	</span>
{/if}
