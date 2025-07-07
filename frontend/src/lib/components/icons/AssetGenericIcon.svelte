<script lang="ts">
	import { Pyramid } from 'lucide-svelte'
	import type { AssetKind } from '../assets/lib'
	import AssetResIcon from './AssetResIcon.svelte'
	import AssetS3Icon from './AssetS3Icon.svelte'
	import AssetVarIcon from './AssetVarIcon.svelte'

	interface Props {
		size?: string
		fill?: string
		assetKind: AssetKind
		class?: string
	}

	let { assetKind, fill, size, class: className = '' }: Props = $props()
</script>

{#if assetKind == 's3object'}
	<AssetS3Icon {fill} width={size} height={size} class={className} />
{:else if assetKind == 'resource'}
	<AssetResIcon {fill} width={size} height={size} class={className} />
{:else if assetKind == 'variable'}
	<AssetVarIcon {fill} width={size} height={size} class={className} />
{:else}
	<Pyramid {fill} {size} class={'fill-none ' + className} />
{/if}
