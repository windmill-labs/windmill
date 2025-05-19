<script lang="ts">
	import { emptyString } from '$lib/utils'
	import { type Trigger } from './utils'
	import { twMerge } from 'tailwind-merge'

	let { trigger }: { trigger: Trigger } = $props()

	function getLabel() {
		if (trigger.type === 'webhook') {
			return 'Webhooks'
		} else if (trigger.type === 'email') {
			return 'Emails'
		} else if (trigger.type === 'cli') {
			return 'CLI'
		} else if (
			trigger.type === 'http' &&
			(!emptyString(trigger.draftConfig?.route_path) ||
				!emptyString(trigger.lightConfig?.http_method))
		) {
			return `${(trigger.draftConfig?.http_method ?? trigger.lightConfig?.http_method ?? 'post').toUpperCase()} ${trigger.draftConfig?.route_path ?? trigger.lightConfig?.route_path}`
		} else if (trigger.isDraft && trigger.draftConfig?.path) {
			return `${trigger.draftConfig?.path}`
		} else if (trigger.isDraft) {
			return `New ${trigger.type.replace(/s$/, '')} trigger`
		} else {
			return trigger.path
		}
	}

	let label = $derived.by(getLabel)
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
