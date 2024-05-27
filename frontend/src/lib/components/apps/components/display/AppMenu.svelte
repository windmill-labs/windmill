<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components, type ButtonComponent } from '../../editor/component'
	import type {
		AppViewerContext,
		BaseAppComponent,
		ComponentCustomCSS,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { Button } from '$lib/components/common'
	import { loadIcon } from '../icon'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import Menu from '$lib/components/common/menu/MenuV2.svelte'
	import { AppButton } from '../buttons'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'menucomponent'> | undefined = undefined
	export let render: boolean
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let menuItems: (BaseAppComponent & ButtonComponent)[]

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, id, {
		result: {
			latestButtonClicked: undefined as string | undefined
		}
	})

	const resolvedConfig = initConfig(
		components['menucomponent'].initialData.configuration,
		configuration
	)

	let css = initCss($app.css?.menucomponent, customCss)

	let beforeIconComponent: any
	let afterIconComponent: any

	$: resolvedConfig.beforeIcon && beforeIconComponent && handleBeforeIcon()
	$: resolvedConfig.afterIcon && afterIconComponent && handleAfterIcon()

	async function handleBeforeIcon() {
		if (resolvedConfig.beforeIcon) {
			beforeIconComponent = await loadIcon(
				resolvedConfig.beforeIcon,
				beforeIconComponent,
				14,
				undefined,
				undefined
			)
		}
	}

	async function handleAfterIcon() {
		if (resolvedConfig.afterIcon) {
			afterIconComponent = await loadIcon(
				resolvedConfig.afterIcon,
				afterIconComponent,
				14,
				undefined,
				undefined
			)
		}
	}
</script>

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
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		<Menu placement="bottom-end" justifyEnd={false} on:close on:open>
			<div slot="trigger">
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
						{#if resolvedConfig.beforeIcon}
							{#key resolvedConfig.beforeIcon}
								<div class="min-w-4" bind:this={beforeIconComponent} />
							{/key}
						{/if}
						{#if resolvedConfig.label && resolvedConfig.label?.length > 0}
							<div>{resolvedConfig.label}</div>
						{/if}
						{#if resolvedConfig.afterIcon}
							{#key resolvedConfig.afterIcon}
								<div class="min-w-4" bind:this={afterIconComponent} />
							{/key}
						{/if}
					</span>
				</Button>
			</div>

			<div class="flex flex-col w-full p-1 gap-2">
				{#if menuItems.length > 0}
					{#each menuItems as actionButton, actionIndex (actionButton?.id)}
						{#if actionButton.type == 'buttoncomponent'}
							<div
								on:pointerup={() => {
									outputs?.result.set({
										latestButtonClicked: actionButton.id
									})
								}}
							>
								<AppButton
									extraKey={'idx' + actionIndex}
									{render}
									id={actionButton.id}
									customCss={actionButton.customCss}
									configuration={actionButton.configuration}
									recomputeIds={actionButton.recomputeIds}
									componentInput={actionButton.componentInput}
									noWFull={false}
									isMenuItem={true}
								/>
							</div>
						{/if}
					{/each}
				{/if}
			</div>
		</Menu>
	</AlignWrapper>
{/if}
