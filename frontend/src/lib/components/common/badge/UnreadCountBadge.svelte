<script lang="ts">
	/**
	 * How many messages have arrived somewhere since it was last read.
	 *
	 * Sits in the row's own flow by default, which is where a list uses it; `class` pins it
	 * to a corner for a caller that has one icon standing for the whole list. Nothing renders
	 * at zero — an absent badge is what "nothing new" looks like.
	 */
	import { twMerge } from 'tailwind-merge'

	interface Props {
		count: number
		/** What the count is of, for the label a screen reader reads. */
		noun?: string
		/** Positioning for a caller that pins it to a corner rather than letting it sit in
		 * the row — the collapsed rail, where the count belongs to an icon button. */
		class?: string
		/** The 12px form, for a corner where the row-sized badge would crowd the icon. */
		small?: boolean
	}

	let { count, noun = 'message', class: className = '', small = false }: Props = $props()
</script>

{#if count > 0}
	<span
		class={twMerge(
			'unread-badge inline-flex items-center justify-center rounded-full bg-surface-accent-primary text-white font-medium',
			small ? 'min-w-3 h-3 px-0.5 text-[8px]' : 'min-w-3.5 h-3.5 px-1 text-[9px]',
			// After the size, not before: tailwind-merge counts a text size as resetting
			// line-height, so a `leading-*` ahead of one is dropped from the result.
			'leading-none',
			className
		)}
		aria-label="{count} unread {noun}{count === 1 ? '' : 's'}"
	>
		{count > 9 ? '9+' : count}
	</span>
{/if}

<style>
	/*
	 * Centring a digit by flex centres its *line box*, which is the font's em box — and a
	 * digit's ink does not sit in the middle of that. Inter reserves descender space a
	 * figure never uses, so the glyph lands fractionally low; at these sizes that reads as
	 * the badge being a pixel off. Trimming the box to cap-height and baseline makes the
	 * ink itself what gets centred. Dropped silently where it is unsupported, which leaves
	 * the same near-miss as before rather than anything worse.
	 */
	.unread-badge {
		text-box: trim-both cap alphabetic;
	}
</style>
