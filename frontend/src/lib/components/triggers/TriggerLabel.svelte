<script lang="ts">
	import { type Trigger } from './utils'

	let { trigger }: { trigger: Trigger } = $props()
</script>

<span class={trigger.isDraft ? 'text-frost-400 italic' : 'font-normal'}>
	{trigger.isDraft
		? (trigger.config?.path ?? `New ${trigger.type.replace(/s$/, '')} trigger`)
		: trigger.path === ''
			? trigger.type === 'webhook'
				? 'Webhooks'
				: 'Emails'
			: trigger.path}
</span>

{#if trigger.isPrimary}
	<span
		class="ml-2 bg-blue-50 dark:bg-blue-900/40 px-1.5 py-0.5 rounded text-xs text-blue-700 dark:text-blue-100"
	>
		Primary
	</span>
{/if}

{#if trigger.isDraft}
	<span
		class="ml-2 text-2xs bg-frost-100 dark:bg-frost-900 text-frost-800 dark:text-frost-100 px-1.5 py-0.5 rounded"
	>
		Draft
	</span>
{/if}
