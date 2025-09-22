<script lang="ts">
	import { createBubbler, preventDefault } from 'svelte/legacy'

	const bubble = createBubbler()
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import { components, type NavbarItem } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import AppNavbarItem from './AppNavbarItem.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'navbarcomponent'> | undefined
		render: boolean
		navbarItems?: NavbarItem[]
	}

	let { id, configuration, customCss = undefined, render, navbarItems = [] }: Props = $props()

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = $state(
		initConfig(components['navbarcomponent'].initialData.configuration, configuration)
	)

	let output = $state(
		initOutput($worldStore, id, {
			result: {
				currentPath: undefined as string | undefined
			}
		})
	)

	let css = $state(initCss($app.css?.navbarcomponent, customCss))
</script>

{#each Object.keys(components['navbarcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.navbarcomponent}
	/>
{/each}

<InitializeComponent {id} />
{#if render}
	<div
		class={twMerge(
			resolvedConfig?.orientation === 'horizontal'
				? 'flex flex-row w-full items-center border-b px-4 gap-4 h-12'
				: 'flex flex-col h-full items-start border-r px-8 gap-2 w-56 mt-4'
		)}
	>
		{#if resolvedConfig.logo?.selected === 'yes'}
			<img
				onpointerdown={preventDefault(bubble('pointerdown'))}
				src={resolvedConfig.logo?.configuration?.yes?.sourceKind == 'png encoded as base64'
					? 'data:image/png;base64,' + resolvedConfig.logo?.configuration?.yes?.source
					: resolvedConfig.logo?.configuration?.yes?.sourceKind == 'jpeg encoded as base64'
						? 'data:image/jpeg;base64,' + resolvedConfig.logo?.configuration?.yes?.source
						: resolvedConfig.logo?.configuration?.yes?.sourceKind == 'svg encoded as base64'
							? 'data:image/svg+xml;base64,' + resolvedConfig.logo?.configuration?.yes?.source
							: resolvedConfig.logo?.configuration?.yes?.source}
				alt={resolvedConfig.logo?.configuration?.yes?.altText}
				style={css?.image?.style ?? ''}
				class={twMerge(`w-auto h-8`, css?.image?.class, 'wm-image')}
			/>
		{/if}
		<div class="font-semibold">
			{resolvedConfig?.title ?? 'No Title'}
		</div>
		<div
			class={twMerge(
				resolvedConfig?.orientation === 'horizontal'
					? 'flex flex-row gap-4 overflow-x-auto'
					: 'flex flex-col gap-4 overflow-y-auto'
			)}
		>
			{#each navbarItems ?? [] as navbarItem, index (index)}
				<Popover notClickable disablePopup={!Boolean(navbarItem.caption)}>
					{#snippet text()}
						{navbarItem.caption}
					{/snippet}
					<AppNavbarItem
						{navbarItem}
						{id}
						borderColor={resolvedConfig?.borderColor}
						{index}
						bind:output
						orientation={resolvedConfig?.orientation}
					/>
				</Popover>
			{/each}
		</div>
	</div>
{/if}
