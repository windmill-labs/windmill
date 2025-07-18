<script lang="ts">
	import { getContext, untrack } from 'svelte'
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
	import { AppButton } from '../buttons'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { Menubar, Menu, MeltButton } from '$lib/components/meltComponents'

	interface Props {
		id: string
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'menucomponent'> | undefined
		render: boolean
		horizontalAlignment?: 'left' | 'center' | 'right' | undefined
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		menuItems: (BaseAppComponent & ButtonComponent)[]
	}

	let {
		id,
		configuration,
		customCss = undefined,
		render,
		horizontalAlignment = undefined,
		verticalAlignment = undefined,
		menuItems
	}: Props = $props()

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, id, {
		result: {
			latestButtonClicked: undefined as string | undefined
		}
	})

	const resolvedConfig = $state(
		initConfig(components['menucomponent'].initialData.configuration, configuration)
	)

	let css = $state(initCss($app.css?.menucomponent, customCss))

	let beforeIconComponent: any = $state()
	let afterIconComponent: any = $state()

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
	$effect(() => {
		resolvedConfig.beforeIcon && beforeIconComponent && untrack(() => handleBeforeIcon())
	})
	$effect(() => {
		resolvedConfig.afterIcon && afterIconComponent && untrack(() => handleAfterIcon())
	})

	let menu: Menu | undefined = $state()
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
		<Menubar class={resolvedConfig.fillContainer ? 'w-full h-full' : ''}>
			{#snippet children({ createMenu })}
				<Menu
					bind:this={menu}
					{createMenu}
					placement="bottom-end"
					justifyEnd={false}
					class={resolvedConfig.fillContainer ? 'w-full h-full' : ''}
					usePointerDownOutside={true}
					renderContent
				>
					{#snippet triggr({ trigger })}
						<MeltButton meltElement={trigger} class="w-full h-full">
							<Button
								on:pointerdown={(e) => e.stopPropagation()}
								btnClasses={twMerge(
									css?.button?.class,
									'wm-button',
									'wm-menu-button',
									resolvedConfig.fillContainer ? 'w-full h-full' : ''
								)}
								wrapperClasses={twMerge(
									'wm-button-container',
									'wm-menu-button-container',
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
											<div class="min-w-4" bind:this={beforeIconComponent}></div>
										{/key}
									{/if}
									{#if resolvedConfig.label && resolvedConfig.label?.length > 0}
										<div>{resolvedConfig.label}</div>
									{/if}
									{#if resolvedConfig.afterIcon}
										{#key resolvedConfig.afterIcon}
											<div class="min-w-4" bind:this={afterIconComponent}></div>
										{/key}
									{/if}
								</span>
							</Button>
						</MeltButton>
					{/snippet}

					<div class="flex flex-col w-full p-1 gap-2 max-h-[50vh] overflow-y-auto">
						{#if menuItems.length > 0}
							{#each menuItems as actionButton, actionIndex (actionButton?.id)}
								{#if actionButton.type == 'buttoncomponent'}
									<div
										onpointerup={() => {
											outputs?.result.set({
												latestButtonClicked: actionButton.id
											})
										}}
									>
										<AppButton
											noInitialize
											extraKey={'idx' + actionIndex}
											render={true}
											id={actionButton.id}
											customCss={actionButton.customCss}
											configuration={actionButton.configuration}
											recomputeIds={actionButton.recomputeIds}
											componentInput={actionButton.componentInput}
											noWFull={false}
											isMenuItem={true}
											onDone={() => {
												menu?.close()
											}}
										/>
									</div>
								{/if}
							{/each}
						{/if}
					</div>
				</Menu>
			{/snippet}
		</Menubar>
	</AlignWrapper>
{/if}
