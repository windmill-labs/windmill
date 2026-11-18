<script lang="ts">
	import { ImageOff } from 'lucide-svelte'
	import { safeHref } from '$lib/utils/safeHref'
	import { hasUntrustedImages, isLocalImageSrc } from './untrustedImages'

	type Props = {
		src?: string
		alt?: string
		title?: string
		width?: string | number
		height?: string | number
	}
	let { src, alt, title, width, height }: Props = $props()

	const untrusted = hasUntrustedImages()
	const blocked = $derived(untrusted && !isLocalImageSrc(src))
	const href = $derived(blocked ? safeHref(src, window.location.href) : undefined)
	const label = $derived(alt || title || src || 'image')
</script>

{#if !blocked}
	<img {src} {alt} {title} {width} {height} />
{:else if href}
	<a
		{href}
		target="_blank"
		rel="noopener noreferrer"
		class="inline-flex items-center gap-1"
		title="Image from {href}, not loaded here"
	>
		<ImageOff size={12} class="shrink-0" />{label}
	</a>
{:else}
	<span class="inline-flex items-center gap-1 text-tertiary">
		<ImageOff size={12} class="shrink-0" />{label}
	</span>
{/if}
