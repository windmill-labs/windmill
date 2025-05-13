<script lang="ts">
	import { type Trigger } from './utils'
	import { twMerge } from 'tailwind-merge'

	let { trigger, discard }: { trigger: Trigger; discard?: boolean } = $props()

	let label = $derived(
		trigger.type === 'webhook'
			? 'Webhooks'
			: trigger.type === 'email'
				? 'Emails'
				: trigger.type === 'cli'
					? 'CLI'
					: (trigger.draftConfig?.path ??
						trigger.path ??
						`New ${trigger.type.replace(/s$/, '')} trigger`)
	)
</script>

<span
	class={twMerge(
		trigger.isDraft ? 'text-frost-400 italic dark:text-frost-200' : 'font-normal',
		discard ? 'text-gray-500 dark:text-gray-400' : '',
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

{#if discard !== undefined}
	<span
		class={twMerge(
			'ml-2 text-2xs bg-frost-100 dark:bg-frost-900 text-frost-800 dark:text-frost-100 px-1.5 py-0.5 rounded whitespace-nowrap',
			discard
				? 'bg-red-400 text-primary-inverse dark:bg-red-900/40'
				: 'bg-blue-500 text-primary-inverse dark:bg-blue-900/40'
		)}
	>
		{discard ? 'Discard' : 'Deploy'}
	</span>
{/if}
