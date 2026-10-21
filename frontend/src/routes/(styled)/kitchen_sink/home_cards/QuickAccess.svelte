<script lang="ts">
	import type { SectionProps, Variant } from './model'
	import CurrentCards from './variants/CurrentCards.svelte'
	import QuietGrid from './variants/QuietGrid.svelte'
	import SplitColumns from './variants/SplitColumns.svelte'
	import Tiles from './variants/Tiles.svelte'
	import Shelf from './variants/Shelf.svelte'

	interface Props extends SectionProps {
		variant: Variant
	}

	let { variant, ...section }: Props = $props()
</script>

{#if variant === 'current'}
	<CurrentCards {...section} />
{:else if variant === 'quiet'}
	<QuietGrid {...section} look="bare" />
{:else if variant === 'tray'}
	<QuietGrid {...section} look="tray" />
{:else if variant === 'columns'}
	<SplitColumns {...section} />
{:else if variant === 'tiles'}
	<Tiles {...section} />
{:else if variant === 'shelf'}
	<Shelf {...section} />
{:else}
	<QuietGrid {...section} look="embedded" />
{/if}
