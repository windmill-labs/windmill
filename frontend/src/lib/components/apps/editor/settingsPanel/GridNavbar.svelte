<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import PanelSection from './common/PanelSection.svelte'
	import { dragHandle, dragHandleZone } from '@windmill-labs/svelte-dnd-action'
	import { generateRandomString } from '$lib/utils'
	import { GripVertical, Plus, Settings } from 'lucide-svelte'
	import type { NavbarItem } from '../component'
	import NavbarWizard from '$lib/components/wizards/NavbarWizard.svelte'
	import { flip } from 'svelte/animate'

	import Badge from '$lib/components/common/badge/Badge.svelte'
	import ResolveConfig from '../../components/helpers/ResolveConfig.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let navbarItems: NavbarItem[] = []
	export let id: string

	const { appPath } = getContext<AppViewerContext>('AppViewerContext')

	let items = navbarItems.map((tab, index) => {
		return { value: tab, id: generateRandomString(), originalIndex: index }
	})

	$: navbarItems = items.map((item) => item.value)

	function addPath() {
		const emptyAppPath: NavbarItem = {
			disabled: {
				type: 'static',
				value: false,
				fieldType: 'boolean'
			},
			label: {
				type: 'static',
				value: undefined,
				fieldType: 'text'
			},
			path: {
				type: 'static',
				value: undefined,
				fieldType: 'text'
			},
			hidden: {
				type: 'static',
				value: false,
				fieldType: 'boolean'
			}
		}

		items = [
			...items,
			{
				value: emptyAppPath,
				id: generateRandomString(),
				originalIndex: items.length
			}
		]
	}

	function handleConsider(e: CustomEvent): void {
		const { items: newItems } = e.detail
		items = newItems
	}

	function handleFinalize(e: CustomEvent) {
		const { items: newItems } = e.detail

		items = newItems
	}

	let resolvedPaths: string[] = []
	let resolvedLabels: string[] = []
</script>

<PanelSection
	title={`Items ${navbarItems && navbarItems.length > 0 ? `(${navbarItems.length})` : ''}`}
>
	{#if !navbarItems || navbarItems.length == 0}
		<span class="text-xs text-tertiary">No items</span>
	{/if}
	<div class="w-full flex gap-2 flex-col mt-2">
		<section
			use:dragHandleZone={{
				items,
				flipDurationMs: 200,
				dropTargetStyle: {}
			}}
			on:consider={handleConsider}
			on:finalize={handleFinalize}
		>
			{#each items as item, index (item.id)}
				<div class="border rounded-md p-2 mb-2 bg-surface" animate:flip={{ duration: 200 }}>
					<ResolveConfig
						{id}
						key={'label'}
						extraKey={item.id}
						bind:resolvedConfig={resolvedLabels[item.originalIndex]}
						configuration={item.value.label}
					/>

					<div class="w-full flex flex-row gap-2 items-center relative my-1">
						<div
							class="text-xs px-2 border-y flex flex-row items-center border rounded-md h-8 w-full"
						>
							{resolvedLabels[item.originalIndex] ?? 'No label'}
						</div>

						<div class="absolute right-[4.5rem]">
							<CloseButton
								noBg
								small
								on:close={() => {
									items = items.filter((_, i) => i !== index)
								}}
							/>
						</div>
						<NavbarWizard bind:value={items[index].value}>
							<svelte:fragment slot="trigger">
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							</svelte:fragment>
						</NavbarWizard>

						<div class="flex flex-col justify-center gap-2">
							<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
							<!-- svelte-ignore a11y-no-static-element-interactions -->
							<div use:dragHandle class="handle w-4 h-4" aria-label="drag-handle">
								<GripVertical size={16} />
							</div>
						</div>
					</div>
					<ResolveConfig
						{id}
						key={'path'}
						extraKey={item.id}
						bind:resolvedConfig={resolvedPaths[item.originalIndex]}
						configuration={item.value.path}
					/>
					{#if resolvedPaths[item.originalIndex]}
						<div class="text-xs text-tertiary">
							Path: <Badge small>{resolvedPaths[item.originalIndex]}</Badge>
							{#if resolvedPaths[item.originalIndex]?.includes(appPath)}
								<Badge small color="blue"
									>Current app

									<Tooltip class="ml-2 !text-blue-900">
										Clicking on those items will stay in the current app, and change the output of
										the component.
									</Tooltip>
								</Badge>
							{/if}
						</div>
					{:else}
						<div class="text-xs text-red-500">No app path or url selected</div>
					{/if}
				</div>
			{/each}
		</section>
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: Plus }}
			on:click={addPath}
			iconOnly
		/>
	</div>
</PanelSection>
