<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss, transformBareBase64IfNecessary } from '../../utils'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { AlignWrapper } from '../helpers'
	import { Button } from '$lib/components/common'
	import { loadIcon } from '../icon'
	import ComponentErrorHandler from '../helpers/ComponentErrorHandler.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'downloadcomponent'> | undefined = undefined
	export let render: boolean
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let noWFull = false

	const resolvedConfig = initConfig(
		components['downloadcomponent'].initialData.configuration,
		configuration
	)

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

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

	$: css = concatCustomCss($app.css?.downloadcomponent, customCss)
</script>

<InitializeComponent {id} />

{#each Object.keys(components['downloadcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#if render}
	<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
		<ComponentErrorHandler
			hasError={resolvedConfig?.source != undefined && typeof resolvedConfig.source !== 'string'}
		>
			<Button
				on:pointerdown={(e) => e.stopPropagation()}
				btnClasses={twMerge(
					css?.button?.class,
					resolvedConfig.fillContainer ? 'w-full h-full' : ''
				)}
				wrapperClasses={resolvedConfig.fillContainer ? 'w-full h-full' : ''}
				style={css?.button?.style}
				disabled={resolvedConfig.source == undefined}
				size={resolvedConfig.size}
				color={resolvedConfig.color}
				download={resolvedConfig.filename}
				href={transformBareBase64IfNecessary(resolvedConfig.source)}
				target="_self"
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
		</ComponentErrorHandler>
	</AlignWrapper>
{/if}
