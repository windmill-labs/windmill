<script lang="ts">
	import { type Trigger } from './utils'
	import { twMerge } from 'tailwind-merge'

	let { trigger }: { trigger: Trigger } = $props()

	let label = $derived(
		trigger.type === 'webhook'
			? 'Webhooks'
			: trigger.type === 'email'
				? 'Emails'
				: (trigger.draftConfig?.path ??
					trigger.path ??
					`New ${trigger.type.replace(/s$/, '')} trigger`)
	)
</script>

<span
	class={twMerge(trigger.isDraft ? 'text-frost-400 italic' : 'font-normal', 'truncate')}
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

{#if trigger.isDraft}
	<span
		class="ml-2 text-2xs bg-frost-100 dark:bg-frost-900 text-frost-800 dark:text-frost-100 px-1.5 py-0.5 rounded whitespace-nowrap"
	>
		Draft only
	</span>
{:else if trigger.draftConfig}
	<span
		class="ml-2 text-2xs bg-frost-100 dark:bg-frost-900 text-frost-800 dark:text-frost-100 px-1.5 py-0.5 rounded whitespace-nowrap"
	>
		+Draft
	</span>
{/if}
