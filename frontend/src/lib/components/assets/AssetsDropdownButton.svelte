<script lang="ts">
	import { clone, pluralize } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import { AlertTriangle, Edit2, Pyramid } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Popover } from '../meltComponents'
	import S3FilePicker from '../S3FilePicker.svelte'
	import ExploreAssetButton, { assetCanBeExplored } from '../ExploreAssetButton.svelte'
	import { formatAssetKind, type Asset } from './lib'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import { untrack } from 'svelte'
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Button from '../common/button/Button.svelte'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import type { Placement } from '@floating-ui/core'

	let {
		assets,
		enableChangeAnimation = true,
		size = 'xs',
		noBtnText = false,
		popoverPlacement = 'bottom-end',
		disableLiTooltip = false,
		onHoverLi,
		liSubtitle
	}: {
		assets: Asset[]
		enableChangeAnimation?: boolean
		size?: 'xs' | '3xs'
		noBtnText?: boolean
		popoverPlacement?: Placement
		disableLiTooltip?: boolean
		onHoverLi?: (asset: Asset, eventType: 'enter' | 'leave') => void
		liSubtitle?: (asset: Asset) => string
	} = $props()

	let prevAssets = $state<typeof assets>([])
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
		assets
		untrack(() => {
			if (deepEqual(assets, prevAssets)) return
			prevAssets = clone(assets)

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
	floatingConfig={{ strategy: 'absolute', placement: popoverPlacement }}
	usePointerDownOutside
	closeOnOtherPopoverOpen
	bind:isOpen
	escapeBehavior="ignore"
>
	<svelte:fragment slot="trigger">
		<div
			class={twMerge(
				size === '3xs' ? 'h-[1.6rem]' : 'py-1.5',
				'text-xs flex items-center gap-1.5 px-2 rounded-md relative',
				'border border-tertiary/30',
				'bg-surface hover:bg-surface-hover active:bg-surface',
				'transition-all hover:text-primary cursor-pointer'
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
				<li
					class="text-sm px-4 h-12 flex gap-4 items-center justify-between hover:bg-surface-hover"
					onmouseenter={() => onHoverLi?.(asset, 'enter')}
					onmouseleave={() => onHoverLi?.(asset, 'leave')}
				>
					<div class="flex flex-col">
						<Tooltip class="select-none max-w-48 truncate" disablePopup={disableLiTooltip}>
							{asset.path}
							<svelte:fragment slot="text">
								{asset.path}
							</svelte:fragment>
						</Tooltip>
						<span class="text-xs text-tertiary select-none">
							{liSubtitle?.(asset) ??
								formatAssetKind({
									...asset,
									metadata: { resource_type: resourceDataCache[asset.path] }
								})}
						</span>
					</div>

					<div class="flex gap-2 items-center">
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
								<AlertTriangle size={16} class="text-orange-600 dark:text-orange-500" />
								<svelte:fragment slot="text">Could not find resource</svelte:fragment>
							</Tooltip>
						{/if}
						{#if assetCanBeExplored(asset, { resource_type: resourceDataCache[asset.path] })}
							<ExploreAssetButton
								{asset}
								{s3FilePicker}
								{dbManagerDrawer}
								onClick={() => (isOpen = false)}
								noText
								_resourceMetadata={{ resource_type: resourceDataCache[asset.path] }}
							/>
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	</svelte:fragment>
</Popover>
<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={dbManagerDrawer} />
<ResourceEditorDrawer bind:this={resourceEditorDrawer} />
