<script lang="ts">
	import { Globe } from 'lucide-svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import { faviconUrl } from '$lib/utils/faviconUrl'
	import type { WebSearchSource } from './shared'

	interface Props {
		sources: WebSearchSource[]
	}

	let { sources }: Props = $props()

	// The URLs come from the provider's response: only render absolute http(s)
	// ones — anything else (javascript:, data:, relative) must not become an
	// href. Also dedupes; providers can surface the same page several times.
	const uniqueSources = $derived(
		Array.from(
			new Map(
				sources
					.filter((s) => {
						try {
							return ['http:', 'https:'].includes(new URL(s.url).protocol)
						} catch {
							return false
						}
					})
					.map((s) => [s.url, s])
			).values()
		)
	)

	function hostnameOf(url: string): string {
		try {
			return new URL(url).hostname.replace(/^www\./, '')
		} catch {
			return url
		}
	}

	const failedFavicons = new SvelteSet<string>()
</script>

<div class="space-y-2">
	<span class="text-2xs text-hint">Sources:</span>
	<div class="flex flex-col max-h-40 overflow-y-auto">
		{#each uniqueSources as source (source.url)}
			{@const hostname = hostnameOf(source.url)}
			<a
				href={source.url}
				target="_blank"
				rel="noopener noreferrer"
				title={source.url}
				class="flex items-center gap-2 py-1 px-1.5 rounded hover:bg-surface-hover min-w-0"
			>
				{#if failedFavicons.has(hostname)}
					<Globe class="w-3.5 h-3.5 shrink-0 text-tertiary" />
				{:else}
					<img
						src={faviconUrl(hostname)}
						alt=""
						loading="lazy"
						class="w-3.5 h-3.5 shrink-0 rounded-sm"
						onerror={() => failedFavicons.add(hostname)}
					/>
				{/if}
				<span class="text-2xs text-primary truncate">{source.title ?? hostname}</span>
				{#if source.title}
					<span class="text-2xs text-tertiary truncate shrink-0 max-w-32">{hostname}</span>
				{/if}
			</a>
		{/each}
	</div>
</div>
