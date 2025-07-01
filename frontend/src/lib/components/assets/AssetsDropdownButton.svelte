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

	let {
		assets: assetsUris,
		enableChangeAnimation = true,
		size = 'xs',
		noBtnText = false
	}: {
		assets: string[]
		enableChangeAnimation?: boolean
		size?: 'xs' | '3xs'
		noBtnText?: boolean
	} = $props()
	const assets = $derived(assetsUris.map(parseAsset).filter((x) => !!x) ?? [])

	let prevAssetsUris = $state<string[]>([])
	let blueBgDiv: HTMLDivElement | undefined = $state()

	let s3FilePicker: S3FilePicker | undefined = $state()
	let dbManagerDrawer: DbManagerDrawer | undefined = $state()
	let resourceEditorDrawer: ResourceEditorDrawer | undefined = $state()
	let isOpen = $state(false)

	let resourceDataCache: Record<string, string | undefined> = $state({})

	$effect(() => {
		if (!enableChangeAnimation) {
			if (blueBgDiv) {
				blueBgDiv.classList.remove('animate-fade-out')
			}
		}
	})

	$effect(() => {
		if (!assetsUris) return
		untrack(() => {
			if (deepEqual(assetsUris, prevAssetsUris)) return
			prevAssetsUris = clone(assetsUris)

			// Replay animation
			if (blueBgDiv && enableChangeAnimation) {
				blueBgDiv.classList.add('animate-fade-out')
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

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	usePointerDownOutside
	closeOnOtherPopoverOpen
	bind:isOpen
>
	<svelte:fragment slot="trigger">
		<div
			class={twMerge(
				size === '3xs' ? 'h-[1.6rem]' : 'py-1.5',
				'text-xs flex items-center gap-1.5 px-2 rounded-md border border-tertiary/30 relative',
				'bg-surface hover:bg-surface-hover active:bg-surface',
				'transition-colors hover:text-primary cursor-pointer'
			)}
		>
			<div
				bind:this={blueBgDiv}
				class="absolute pointer-events-none bg-slate-300 dark:bg-[#576278] inset-0 rounded-md opacity-0"
			></div>
			<Pyramid size={size === '3xs' ? 13 : 16} class="z-10" />
			<span
				class={twMerge('z-10 font-normal', size === '3xs' ? 'text-3xs mt-[0.08rem]' : 'text-xs')}
			>
				{noBtnText ? assets.length : pluralize(assets.length, 'asset')}
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
