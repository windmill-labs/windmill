<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		RichConfiguration,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { AlignWrapper } from '../helpers'
	import { Button } from '$lib/components/common'
	import { loadIcon } from '../icon'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'menucomponent'> | undefined = undefined
	export let render: boolean
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let menuItems: RichConfiguration | undefined

	const resolvedConfig = initConfig(
		components['menucomponent'].initialData.configuration,
		configuration
	)

	type MenuItems = {
		label: RichConfiguration
	}

	type ResolvedMenuItems = {
		label: string
	}

	let resolvedMenuItems: MenuItems[]

	let resolvedMenuItemsValues: ResolvedMenuItems[]

	let result: any | undefined = undefined

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	let beforeIconComponent: any
	let afterIconComponent: any

	$: resolvedConfig.beforeIcon && handleBeforeIcon()
	$: resolvedConfig.afterIcon && handleAfterIcon()

	async function handleBeforeIcon() {
		if (resolvedConfig.beforeIcon) {
			beforeIconComponent = await loadIcon(resolvedConfig.beforeIcon)
		}
	}

	async function handleAfterIcon() {
		if (resolvedConfig.afterIcon) {
			afterIconComponent = await loadIcon(resolvedConfig.afterIcon)
		}
	}

	let css = initCss($app.css?.menucomponent, customCss)
</script>

{#if menuItems}
	<ResolveConfig
		{id}
		key={'datasets'}
		bind:resolvedConfig={resolvedMenuItems}
		configuration={menuItems}
	/>
{/if}

{#if resolvedMenuItems}
	{#each resolvedMenuItems as resolvedMenuItem, index}
		<ResolveConfig
			{id}
			key={'menu-items' + index}
			bind:resolvedConfig={resolvedMenuItemsValues[index].label}
			configuration={resolvedMenuItem.label}
		/>
	{/each}
{/if}

<InitializeComponent {id} />

{#each Object.keys(components['menucomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.menucomponent}
	/>
{/each}

{#if render}
	{JSON.stringify(resolvedMenuItems)}
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		<Dropdown justifyEnd={false} items={result ?? []}>
			<svelte:fragment slot="buttonReplacement">
				<Button
					on:pointerdown={(e) => e.stopPropagation()}
					btnClasses={twMerge(
						css?.button?.class,
						'wm-button',
						'wm-download-button',
						resolvedConfig.fillContainer ? 'w-full h-full' : ''
					)}
					wrapperClasses={twMerge(
						'wm-button-container',
						'wm-download-button-container',
						resolvedConfig.fillContainer ? 'w-full h-full' : ''
					)}
					style={css?.button?.style}
					size={resolvedConfig.size}
					color={resolvedConfig.color}
					nonCaptureEvent
				>
					<span class="truncate inline-flex gap-2 items-center">
						{#if resolvedConfig.beforeIcon && beforeIconComponent}
							<svelte:component this={beforeIconComponent} size={14} />
						{/if}
						{#if resolvedConfig.label && resolvedConfig.label?.length > 0}
							<div>{resolvedConfig.label}</div>
						{/if}
						{#if resolvedConfig.afterIcon && afterIconComponent}
							<svelte:component this={afterIconComponent} size={14} />
						{/if}
					</span>
				</Button>
			</svelte:fragment>
		</Dropdown>
	</AlignWrapper>
{/if}
