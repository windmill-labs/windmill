<script lang="ts">
	import { BoxesIcon, DatabaseIcon, FileIcon, Pyramid } from 'lucide-svelte'
	import type { AssetKind } from '../assets/lib'
	import DucklakeIcon from './DucklakeIcon.svelte'

	interface Props {
		size?: string
		assetKind: AssetKind
		class?: string
	}

	let { assetKind, size, class: className = '' }: Props = $props()
</script>

{#if assetKind == 's3object'}
	<FileIcon {size} class={className} />
{:else if assetKind == 'datatable'}
	<DatabaseIcon {size} class={className} />
{:else if assetKind == 'ducklake'}
	<DucklakeIcon {size} class={className} />
{:else if assetKind == 'resource'}
	<BoxesIcon {size} class={className} />
{:else}
	<Pyramid {size} class={className} />
{/if}
