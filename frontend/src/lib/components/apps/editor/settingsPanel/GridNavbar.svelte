<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import PanelSection from './common/PanelSection.svelte'
	import { dragHandle, dragHandleZone } from '@windmill-labs/svelte-dnd-action'
	import { generateRandomString } from '$lib/utils'
	import { GripVertical, Plus, Settings } from 'lucide-svelte'
	import type { NavbarItem } from '../component'
	import NavbarWizard from '$lib/components/wizards/NavbarWizard.svelte'

	import Badge from '$lib/components/common/badge/Badge.svelte'
	import ResolveConfig from '../../components/helpers/ResolveConfig.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { StaticAppInput } from '../../inputType'
	import ResolveNavbarItemPath from '../../components/display/ResolveNavbarItemPath.svelte'

	interface Props {
		navbarItems?: NavbarItem[]
		id: string
	}

	let { navbarItems = $bindable(), id }: Props = $props()

	$effect.pre(() => {
		if (!navbarItems) {
			navbarItems = []
		}
	})

	const { appPath } = getContext<AppViewerContext>('AppViewerContext')

	let items = $state(
		(navbarItems ?? []).map((tab, index) => {
			return { value: tab, id: generateRandomString(), originalIndex: index }
		})
	)

	$effect(() => {
		navbarItems = items.map((item) => item.value)
	})

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
				type: 'oneOf',
				selected: 'app',
				labels: {
					href: 'Navigate to an external URL',
					app: 'Navigate to an app'
				},
				configuration: {
					href: {
						href: {
							type: 'static',
							value: undefined,
							fieldType: 'text',
							tooltip:
								"The URL to navigate to when the item is clicked. Will be opened in a new tab. If you want to navigate to an other app, use the 'App' option."
						}
					},
					app: {
						path: {
							type: 'static',
							value: '',
							fieldType: 'app-path',
							allowTypeChange: false,
							tooltip:
								'The app to navigate to when the item is clicked. Will be opened in the same tab. If you want to navigate to an external URL, use the "Href" option.'
						} as StaticAppInput,
						queryParamsOrHash: {
							type: 'static',
							value: undefined,
							fieldType: 'text',
							tooltip:
								'Query parameters or hash to append to the URL. For example, `?key=value` or `#hash`.',
							placeholder: '?key=value#hash'
						}
					}
				}
			} as const,
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

	let resolvedPaths: string[] = $state([])
	let resolvedLabels: string[] = $state([])
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
			onconsider={handleConsider}
			onfinalize={handleFinalize}
		>
			{#each items as item, index (item.id)}
				{#key item.id}
					<div class="border rounded-md p-2 mb-2 bg-surface">
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
								{#snippet trigger()}
									<Button color="light" size="xs2" nonCaptureEvent={true}>
										<div class="flex flex-row items-center gap-2 text-xs font-normal">
											<Settings size={16} />
										</div>
									</Button>
								{/snippet}
							</NavbarWizard>

							<div class="flex flex-col justify-center gap-2">
								<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div use:dragHandle class="handle w-4 h-4" aria-label="drag-handle">
									<GripVertical size={16} />
								</div>
							</div>
						</div>

						<ResolveNavbarItemPath
							navbarItem={item.value}
							{id}
							{index}
							bind:resolvedPath={resolvedPaths[item.originalIndex]}
						/>

						{#if resolvedPaths[item.originalIndex]}
							<div class="text-xs text-tertiary flex gap-2 flex-row flex-wrap">
								Path: <Badge small>{resolvedPaths[item.originalIndex]}</Badge>
								{#if $appPath && resolvedPaths[item.originalIndex]?.includes($appPath)}
									<Badge small color="blue"
										>Current app

										<Tooltip class="ml-2 !text-blue-900">
											Clicking on those items will keep you in the current tab and change the output
											of the component.
										</Tooltip>
									</Badge>
								{/if}
							</div>
						{:else}
							<div class="text-xs text-red-500">No app path or url selected</div>
						{/if}
					</div>
				{/key}
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
