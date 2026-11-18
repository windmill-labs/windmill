<!--
@component
The pen that opens an item's summary and path for editing, beside the breadcrumb that shows them.

One affordance for both popovers behind it: an editor's, which edits the draft the next deploy
writes, and a deployed page's, which saves straight away. They look the same because they mean
the same thing to a reader — "rename this".
-->
<script lang="ts">
	import { Pencil } from 'lucide-svelte'
	import { Button } from '$lib/components/common'

	interface Props {
		/** Names what the popover offers, for the tooltip and the accessible name. */
		label: string
		/** `hover` keeps the pen out of the way until the row is hovered, which is what the band
		 *  wants beside a name; `always` is for a name with nothing else to point at, such as an
		 *  item with no summary yet. */
		visibility?: 'hover' | 'always'
		/** Held visible while the popover it opens is open, whatever `visibility` says. */
		open?: boolean
	}

	let { label, visibility = 'hover', open = false }: Props = $props()
</script>

<Button
	variant="subtle"
	unifiedSize="sm"
	iconOnly
	startIcon={{ icon: Pencil }}
	title={label}
	aria-label={label}
	btnClasses={visibility === 'hover' && !open
		? 'opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100'
		: ''}
/>
