<script lang="ts">
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

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'navbarcomponent'> | undefined = undefined
	export let render: boolean
	export let navbarItems: NavbarItem[] = []

	const { app, worldStore, appPath } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['navbarcomponent'].initialData.configuration,
		configuration
	)

	initOutput($worldStore, id, {})

	let css = initCss($app.css?.navbarcomponent, customCss)
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
	<div class="flex flex-row w-full items-center border-b px-4 gap-4">
		{#if resolvedConfig.source !== undefined}
			<img
				on:pointerdown|preventDefault
				src={resolvedConfig.sourceKind == 'png encoded as base64'
					? 'data:image/png;base64,' + resolvedConfig.source
					: resolvedConfig.sourceKind == 'jpeg encoded as base64'
					? 'data:image/jpeg;base64,' + resolvedConfig.source
					: resolvedConfig.sourceKind == 'svg encoded as base64'
					? 'data:image/svg+xml;base64,' + resolvedConfig.source
					: resolvedConfig.source}
				alt={resolvedConfig.altText}
				style={css?.image?.style ?? ''}
				class={twMerge(`w-auto h-8`, css?.image?.class, 'wm-image')}
			/>
		{/if}
		<div class="font-semibold">
			{resolvedConfig?.title ?? 'No Title'}
		</div>
		<div class="flex flex-row gap-4 overflow-x-auto">
			{#each navbarItems ?? [] as navbarItem}
				{#if navbarItem.path || !navbarItem.hidden}
					<Popover notClickable disablePopup={!Boolean(navbarItem.caption)}>
						<svelte:fragment slot="text">{navbarItem.caption}</svelte:fragment>
						<div
							class={twMerge(
								'py-2',
								appPath === navbarItem.path ? 'border-b-2 border-gray-500 ' : ''
							)}
							style={`border-color: ${resolvedConfig?.borderColor ?? 'transparent'}`}
						>
							<AppNavbarItem {navbarItem} />
						</div>
					</Popover>
				{/if}
			{/each}
		</div>
	</div>
{/if}
