<script lang="ts">
	import { Globe } from 'lucide-svelte'
	import { SvelteSet } from 'svelte/reactivity'
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

	// Favicons come from Google's public favicon service, which discloses each
	// consulted hostname to a third party from the user's browser — an accepted
	// tradeoff for now (blocked/air-gapped environments degrade to the Globe
	// icon via onerror). Hit gstatic directly rather than www.google.com/s2/
	// favicons: the app is served with COEP require-corp, and the s2 redirect
	// hop carries no Cross-Origin-Resource-Policy header, so the browser blocks
	// the image. The gstatic endpoint itself responds with CORP: cross-origin.
	function faviconUrl(hostname: string): string {
		return `https://t3.gstatic.com/faviconV2?client=SOCIAL&type=FAVICON&fallback_opts=TYPE,SIZE,URL&url=https://${encodeURIComponent(hostname)}&size=64`
	}
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
