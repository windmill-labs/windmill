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

	export let navbarItems: NavbarItem[] = []
	export let id: string

	//const { appPath } = getContext<AppViewerContext>('AppViewerContext')

	let items = navbarItems.map((tab, index) => {
		return { value: tab, id: generateRandomString(), originalIndex: index }
	})

	$: navbarItems = items.map((item) => item.value)

	function addPath() {
		const emptyAppPath: NavbarItem = {
			disabled: false,
			label: undefined,
			path: {
				type: 'static',
				value: undefined,
				fieldType: 'select',
				selectOptions: []
			},
			hidden: false
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
					<div class="w-full flex flex-row gap-2 items-center relative my-1">
						<div class="flex flex-row rounded-md bg-surface items-center h-full">
							<div class="relative w-full">
								<input
									class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
									bind:value={items[index].value.label}
									placeholder="Field"
								/>
							</div>
						</div>

						<div class="absolute right-24">
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
						key={'path' + id}
						bind:resolvedConfig={resolvedPaths[index]}
						configuration={item.value.path}
					/>
					{#if resolvedPaths[index]}
						<div class="text-xs text-tertiary">
							Path: <Badge small>{resolvedPaths[index]}</Badge>
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
