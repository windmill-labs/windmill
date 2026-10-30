<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { roleCapabilities } from '$lib/stores'

	interface Props {
		label?: string
		class?: string
	}

	let { label = 'Manage groups', class: clazz = undefined }: Props = $props()
</script>

<!-- Groups have no entry in the main menu, so every place that picks one links to their page.
     It opens in a new tab: the callers are forms whose unsaved state a navigation would drop. -->
{#if $roleCapabilities.visibility.groups}
	<a
		href="/groups"
		target="_blank"
		class={twMerge(
			'inline-flex items-center gap-1 text-2xs font-normal text-accent hover:underline whitespace-nowrap',
			clazz
		)}
	>
		{label}
		<ExternalLink size={12} />
	</a>
{/if}
