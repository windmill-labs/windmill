<script lang="ts">
	import { clone, pluralize } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import { AlertTriangle, Edit2, Pyramid } from 'lucide-svelte'
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
	import Button from '../common/button/Button.svelte'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'

	let { assets: assetsUris }: { assets: string[] } = $props()
	const assets = $derived(assetsUris.map(parseAsset).filter((x) => !!x) ?? [])

	let prevAssetsUris = $state<string[]>([])
	let blueBgDiv: HTMLDivElement | undefined = $state()

	let s3FilePicker: S3FilePicker | undefined = $state()
	let dbManagerDrawer: DbManagerDrawer | undefined = $state()
	let resourceEditorDrawer: ResourceEditorDrawer | undefined = $state()
	let isOpen = $state(false)

	let resourceDataCache: Record<string, string | undefined> = $state({})

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
				if (asset.kind !== 'resource' || asset.path in resourceDataCache) continue
				ResourceService.getResource({ path: asset.path, workspace: $workspaceStore! })
					.then((resource) => (resourceDataCache[asset.path] = resource.resource_type))
					.catch((err) => (resourceDataCache[asset.path] = undefined))
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
					<li class="text-sm px-4 h-12 flex gap-4 items-center justify-between">
						<Tooltip class="select-none max-w-48 truncate">
							{formatAsset(asset)}
							<svelte:fragment slot="text">
								{formatAsset(asset)}
							</svelte:fragment>
						</Tooltip>

						<div class="flex gap-2">
							{#if asset.kind === 'resource' && resourceDataCache[asset.path] !== undefined}
								<Button
									startIcon={{ icon: Edit2 }}
									size="xs"
									variant="border"
									spacingSize="xs2"
									iconOnly
									on:click={() => (resourceEditorDrawer?.initEdit(asset.path), (isOpen = false))}
								/>
							{/if}
							{#if asset.kind === 'resource' && resourceDataCache[asset.path] === undefined}
								<Tooltip class="mr-2.5">
									<AlertTriangle size={16} class="text-orange-500" />
									<svelte:fragment slot="text">Could not fetch resource</svelte:fragment>
								</Tooltip>
							{/if}
							{#if assetCanBeExplored(asset, { resourceType: resourceDataCache[asset.path] })}
								<ExploreAssetButton
									{asset}
									{s3FilePicker}
									{dbManagerDrawer}
									onClick={() => (isOpen = false)}
									noText
									_resourceMetadata={{ resourceType: resourceDataCache[asset.path] }}
								/>
							{/if}
						</div>
					</li>
				{/if}
			{/each}
		</ul>
	</svelte:fragment>
</Popover>
<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={dbManagerDrawer} />
<ResourceEditorDrawer bind:this={resourceEditorDrawer} />
