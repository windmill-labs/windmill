<script module lang="ts">
	let openViewers = 0

	/**
	 * Whether an expanded viewer is currently on screen. Components that treat a
	 * click elsewhere in the document as "dismiss me" MUST consult this: the viewer
	 * portals to `body`, so clicking it is not DOM-contained by whatever opened it
	 * and would otherwise read as an outside-click — closing the thing underneath.
	 */
	export function isImageViewerOpen(): boolean {
		return openViewers > 0
	}
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Modal2 from '../modal/Modal2.svelte'

	interface Props {
		/** Source of the inline thumbnail. */
		src: string
		/**
		 * Source for the expanded view; defaults to `src`. Pass the full-resolution
		 * copy when `src` is a bounded thumbnail, so expanding reveals real detail
		 * rather than an upscale of the thumbnail.
		 */
		fullSrc?: string
		/** Names the image, and the expanded view when no `title` is given. */
		alt?: string
		/** Heading of the expanded view; defaults to `alt`. */
		title?: string
		/** Classes for the inline thumbnail — sizing, border, radius. */
		class?: string
	}

	let { src, fullSrc, alt = '', title, class: className = '' }: Props = $props()

	let isOpen = $state(false)

	$effect(() => {
		if (!isOpen) return
		openViewers++
		return () => {
			openViewers--
		}
	})

	function expand(e: Event) {
		// Thumbnails are commonly rendered inside a click target of their own (the
		// AI chat's user bubble opens the message editor on click); expanding must
		// not trigger those too.
		e.stopPropagation()
		isOpen = true
	}
</script>

<!-- The image itself carries the role rather than sitting in a wrapping button:
     callers size it with percentage heights that resolve against their own
     container, and an extra auto-height box between the two would break them.
     `alt` names it and Enter/Space activate it, so the role is honoured. -->
<!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
<img
	{src}
	{alt}
	class={twMerge('cursor-zoom-in', className)}
	role="button"
	tabindex="0"
	title="Click to expand"
	onclick={expand}
	onkeydown={(e) => {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault()
			expand(e)
		}
	}}
/>

<Modal2
	bind:isOpen
	title={title || alt || 'Image'}
	fixedHeight="adaptive"
	css={{ popup: { style: 'width: auto;' } }}
>
	<!-- Capped, never stretched: the image keeps its natural size up to the
	     viewport bound, so a small source gains room without turning blurry. -->
	<img src={fullSrc ?? src} {alt} class="max-h-[75vh] max-w-[80vw] object-contain mx-auto" />
</Modal2>
