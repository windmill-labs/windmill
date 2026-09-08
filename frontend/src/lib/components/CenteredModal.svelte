<script lang="ts">
	import { setLicense } from '$lib/enterpriseUtils'
	import { twMerge } from 'tailwind-merge'
	import { Loader2 } from 'lucide-svelte'
	import LoginPageHeader from './LoginPageHeader.svelte'

	interface Props {
		subtitle?: string | undefined
		/** Rendered under the title, for a subtitle that needs markup (a link, say).
		 * Sits below `subtitle` when both are given. */
		subtitleSnippet?: import('svelte').Snippet
		title?: string
		large?: boolean
		centerVertically?: boolean
		loading?: boolean
		containOverflow?: boolean
		children?: import('svelte').Snippet
	}

	let {
		subtitle = undefined,
		subtitleSnippet = undefined,
		title = 'Windmill',
		large = false,
		centerVertically = true,
		loading = false,
		containOverflow = false,
		children
	}: Props = $props()

	setLicense()

	let height = $state(0)
</script>

<div
	class="flex justify-center h-screen p-4 relative bg-surface-secondary {containOverflow
		? 'overflow-hidden'
		: 'overflow-auto'}"
	class:items-center={centerVertically}
	style="scrollbar-gutter: stable both-edges;"
	bind:clientHeight={height}
>
	<div
		class={twMerge(
			'flex flex-col gap-2 items-center w-full pb-8',
			containOverflow ? 'min-h-0' : 'h-fit',
			containOverflow ? '' : height > 1080 ? 'pt-28' : 'pt-12'
		)}
	>
		<div class="mb-4">
			<!-- The mark moved to the header, so a page that is only waiting (logging out,
				redirecting) needs its own thing that moves. -->
			<h1
				class="flex items-center justify-center gap-2 text-center text-lg text-emphasis font-semibold"
			>
				{title}
				{#if loading}
					<Loader2 size={16} class="animate-spin shrink-0 text-secondary" />
				{/if}
			</h1>
			{#if subtitle}
				<p class="text-xs font-normal text-primary text-center mt-2">
					{subtitle}
				</p>
			{/if}
			{#if subtitleSnippet}
				<div class="text-center mt-2">{@render subtitleSnippet()}</div>
			{/if}
		</div>

		{#if children}
			<div
				class="rounded-md bg-surface w-full {large
					? 'max-w-5xl'
					: 'max-w-[640px]'} p-4 sm:py-8 sm:px-10 z-10 {containOverflow
					? 'flex-1 min-h-0 flex flex-col'
					: ''}"
			>
				{@render children()}
			</div>
		{/if}
	</div>

	<LoginPageHeader />
</div>
