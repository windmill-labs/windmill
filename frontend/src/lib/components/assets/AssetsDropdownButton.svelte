<script lang="ts">
	import { clone, pluralize, truncate } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import { Pyramid } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Popover } from '../meltComponents'
	import S3FilePicker from '../S3FilePicker.svelte'
	import ExploreAssetButton, {
		assetCanBeExplored
	} from '../../../routes/(root)/(logged)/assets/ExploreAssetButton.svelte'
	import { formatAsset, parseAsset } from './lib'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import { untrack } from 'svelte'
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	let { assets: assetsUris }: { assets: string[] } = $props()
	const assets = $derived(assetsUris.map(parseAsset).filter((x) => !!x) ?? [])

	let prevAssetsUris = $state<string[]>([])
	let blueBgDiv: HTMLDivElement | undefined = $state()

	let s3FilePicker: S3FilePicker | undefined = $state()
	let dbManagerDrawer: DbManagerDrawer | undefined = $state()
	let isOpen = $state(false)

	let resourceTypesCache: Record<string, string | undefined> = $state({})

	$effect(() => {
		if (!assetsUris) return
		untrack(() => {
			if (deepEqual(assetsUris, prevAssetsUris)) return
			prevAssetsUris = clone(assetsUris)
			// Replay animation
			if (blueBgDiv) {
				blueBgDiv.style.animation = 'none'
				blueBgDiv.offsetHeight /* trigger reflow */
				blueBgDiv.style.animation = ''
			}
			for (const asset of assets) {
				if (asset.kind !== 'resource' || asset.path in resourceTypesCache) continue
				ResourceService.getResource({ path: asset.path, workspace: $workspaceStore! })
					.then((resource) => (resourceTypesCache[asset.path] = resource.resource_type))
					.catch((err) => (resourceTypesCache[asset.path] = undefined))
			}
		})
	})
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }} bind:isOpen>
	<svelte:fragment slot="trigger">
		<div
			class={twMerge(
				'text-xs flex items-center gap-2 px-2 py-1.5 bg-surface rounded-md border relative',
				'transition-colors hover:bg-surface-hover hover:text-primary active:bg-surface/0 cursor-pointer'
			)}
		>
			<div
				bind:this={blueBgDiv}
				class="absolute pointer-events-none bg-blue-300/60 inset-0 rounded-md opacity-0 animate-fade-out"
			></div>
			<Pyramid size={16} class="z-10" />
			<span class="z-10">
				{pluralize(assets.length, 'asset')}
			</span>
		</div>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<ul class="divide-y rounded-md">
			{#each assets as asset}
				{#if asset}
					<li class="text-sm px-4 h-12 flex gap-4 items-center select-none justify-between">
						{truncate(formatAsset(asset), 40)}
						{#if assetCanBeExplored(asset, { resourceType: resourceTypesCache[asset.path] })}
							<ExploreAssetButton
								{asset}
								{s3FilePicker}
								{dbManagerDrawer}
								onClick={() => (isOpen = false)}
								noText
								_resourceMetadata={{ resourceType: resourceTypesCache[asset.path] }}
							/>
						{/if}
					</li>
				{/if}
			{/each}
		</ul>
	</svelte:fragment>
</Popover>
<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={dbManagerDrawer} />
