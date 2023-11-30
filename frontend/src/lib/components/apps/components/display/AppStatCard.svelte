<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { getImageDataURL, initCss } from '../../utils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { ArrowDown } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { loadIcon } from '../icon'
	import Loader from '../helpers/Loader.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'statcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['statcomponent'].initialData.configuration,
		configuration
	)

	initOutput($worldStore, id, {})

	let css = initCss($app.css?.statcomponent, customCss)

	let iconComponent: any

	$: isIcon && resolvedConfig?.media?.configuration?.icon?.icon && handleIcon()

	async function handleIcon() {
		if (resolvedConfig?.media?.configuration?.icon?.icon) {
			iconComponent = await loadIcon(resolvedConfig.media.configuration.icon.icon)
		}
	}

	$: isIcon = resolvedConfig.media?.selected == 'icon'
</script>

{#each Object.keys(components['statcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.imagecomponent}
	/>
{/each}

<InitializeComponent {id} />

{#if render}
	<div
		class={twMerge(
			'flex flex-row gap-4 items-center p-4 rounded-md shadow-md h-full',
			css?.container?.class,
			'wm-statistic-card-container'
		)}
		style={css?.container?.style}
	>
		<div
			class={twMerge(
				'flex items-center justify-center w-12 h-12 border rounded-md p-2 text-black',
				css?.media?.class,
				'wm-statistic-card-media'
			)}
			style={css?.media?.style}
		>
			{#if isIcon}
				{#if resolvedConfig?.media && iconComponent}
					<svelte:component this={iconComponent} size={24} />
				{/if}
			{:else}
				<Loader loading={resolvedConfig?.media?.configuration?.image?.source == undefined}>
					<img
						on:pointerdown|preventDefault
						src={getImageDataURL(
							resolvedConfig?.media?.configuration?.image?.sourceKind,
							resolvedConfig?.media?.configuration?.image?.source
						)}
						alt={resolvedConfig?.title}
					/>
				</Loader>
			{/if}
		</div>

		<div class="w-full">
			<div
				class={twMerge(
					'font-normal text-primary leading-none',
					css?.title?.class,
					'wm-statistic-card-title'
				)}
				style={css?.title?.style}
			>
				{resolvedConfig?.title}
			</div>
			<div class="mt-1 flex items-baseline justify-between">
				<div
					class={twMerge(
						'flex items-baseline text-2xl leading-none font-semibold text-blue-600 dark:text-blue-200',
						css?.value?.class,
						'wm-statistic-card-value'
					)}
					style={css?.value?.style}
				>
					{resolvedConfig?.value}
				</div>

				{#if resolvedConfig?.progress !== undefined && resolvedConfig?.progress !== null && resolvedConfig?.progress !== 0}
					<div
						class={twMerge(
							'flex items-center flex-row gap-2 rounded-full px-2.5 py-0.5 text-sm font-medium',
							resolvedConfig?.progress > 0
								? 'bg-green-100 text-green-800'
								: 'bg-red-100 text-red-800'
						)}
					>
						<ArrowDown
							size={16}
							class={resolvedConfig?.progress > 0 ? 'transform rotate-180' : 'transform rotate-0'}
						/>
						{resolvedConfig?.progress}%
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
