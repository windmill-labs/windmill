<script lang="ts">
	import { Tab, TabContent } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { getContext, untrack } from 'svelte'
	import type { AppViewerContext, ComponentCssProperty } from '../../types'
	import { ccomponents, components, type AppComponent } from '../component'
	import CssProperty from '../componentsPanel/CssProperty.svelte'
	import { quickStyleProperties } from '../componentsPanel/quickStyleProperties'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { customisationByComponent, hasStyleValue } from '../componentsPanel/cssUtils'
	import CssMigrationModal from './CSSMigrationModal.svelte'
	import CssPropertyWrapper from './CssPropertyWrapper.svelte'
	import { onMount } from 'svelte'
	import { findGridItemById } from '../appUtilsCore'

	const { app, cssEditorOpen, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	let tab: 'local' | 'global' = $state('local')
	let overrideGlobalCSS: (() => void) | undefined = $state(undefined)
	let overrideLocalCSS: (() => void) | undefined = $state(undefined)

	let component: AppComponent | undefined = $derived(
		findGridItemById($app.grid, $app.subgrids, $selectedComponent?.[0] ?? '')?.data
	)

	$effect.pre(() => {
		if (
			(component && component?.customCss === undefined) ||
			(Object.keys(component?.customCss ?? {}).length === 0 &&
				Object.keys(ccomponents[component?.type ?? '']?.customCss ?? {}).length > 0)
		) {
			untrack(() => {
				let oldComponent = findGridItemById($app.grid, $app.subgrids, $selectedComponent?.[0] ?? '')
				if (oldComponent) {
					oldComponent.data = {
						...(oldComponent.data ?? {}),
						customCss: JSON.parse(
							JSON.stringify(ccomponents[component?.type ?? '']?.customCss ?? {})
						)
					}
				}
			})
			app.set($app)
		}
	})

	let type = $derived(component?.type)
	let migrationModal: CssMigrationModal | undefined = $state(undefined)

	let customCssByComponentType = $derived(
		component?.type && $app.css
			? Object.entries($app.css[component.type] || {}).map(([id, v]) => ({
					id,
					forceStyle: v?.style != undefined,
					forceClass: v?.['class'] != undefined
				}))
			: undefined
	)

	function copyLocalToGlobal(name: string, value: ComponentCssProperty | undefined) {
		if (!value) {
			sendUserToast('No local CSS to copy')
		} else {
			const type = component?.type

			if (!type) return

			if (hasStyleValue($app.css?.[type]?.[name])) {
				overrideGlobalCSS = () => {
					$app.css![type]![name] = JSON.parse(JSON.stringify(value))
					app.set($app)
				}
			} else {
				if (!$app.css![type]) {
					initGlobalCss()
				}

				$app.css![type]![name] = JSON.parse(JSON.stringify(value))
				app.set($app)
				sendUserToast('Global CSS copied')
			}
		}
	}

	function copyGlobalToLocal(id: string, value: any) {
		if (!value) {
			sendUserToast('No global CSS to copy')
		} else {
			if (hasStyleValue(value)) {
				overrideLocalCSS = () => {
					component!.customCss![id] = JSON.parse(JSON.stringify(value))
					app.set($app)
				}
			} else {
				component!.customCss![id] = JSON.parse(JSON.stringify(value))
				app.set($app)
				sendUserToast('Local CSS copied')
			}
		}
	}

	function initGlobalCss() {
		// If the global css is not initialised, we initialise it.
		// Should only happen once per app
		if (!$app.css) {
			$app.css = {}
		}

		// If the global css for this component type is not initialised, we initialise it.
		// Should only happen once per component type
		if (
			$app.css &&
			component &&
			!$app.css[component.type]?.style &&
			components[component.type] &&
			$app.css[component.type] === undefined
		) {
			$app.css[component.type] = JSON.parse(JSON.stringify(components[component.type].customCss))
			app.set($app)
		}
	}

	function getSelector(key: string) {
		return customisationByComponent
			.find((c) => c.components.includes(component?.type ?? ''))
			?.selectors.find((s) => {
				return s.customCssKey === key
			})?.selector
	}

	onMount(() => {
		initGlobalCss()
	})
</script>

{#if component}
	{#key component?.id}
		<div class="px-2 flex gap-1 flex-col w-full pb-4">
			{#if !$cssEditorOpen}
				<Button
					size="xs2"
					variant="default"
					on:click={() => {
						$cssEditorOpen = true
					}}
				>
					<div class="flex flex-row gap-1 text-xs items-center">
						Open theme editor{$enterpriseLicense === undefined ? '  (EE only)' : ''}
						<Tooltip light>
							You can also use the App CSS Editor to customise the CSS of all components.
						</Tooltip>
					</div>
				</Button>
			{:else}
				<div></div>
			{/if}

			{#if $enterpriseLicense !== undefined}
				<Button
					size="xs2"
					variant="default"
					on:click={() => {
						migrationModal?.open()
					}}
				>
					Convert to global CSS
				</Button>
			{/if}
		</div>

		<Tabs bind:selected={tab}>
			<Tab value="local">
				{#snippet extra()}
					<div class="flex flex-row gap-2 items-center">
						ID
						<Badge color="indigo" size="xs">
							{component?.id}
						</Badge>

						<Tooltip light>
							You can customise the CSS and the classes of this component instance. These
							customisations will only be applied to this component. You can also apply custom
							classes set on the Global styling panel.
						</Tooltip>
					</div>
				{/snippet}
			</Tab>
			<Tab value="global">
				{#snippet extra()}
					<div class="flex flex-row gap-2 items-center">
						Global: {type ? ccomponents[type].name : ''}

						<Tooltip light>
							You can customise the CSS and the classes of all components of this type. These
							customisations will be applied to all components of this type.
						</Tooltip>
					</div>
				{/snippet}
			</Tab>
			{#snippet content()}
				<TabContent value="local">
					{#if component && component.customCss !== undefined}
						<div class="flex-col flex gap-4 divide-y">
							{#each Object.keys(ccomponents[component.type].customCss ?? {}) as name}
								<div class="w-full">
									<CssProperty
										quickStyleProperties={quickStyleProperties?.[component.type]?.[name]}
										forceStyle={ccomponents[component.type].customCss[name].style !== undefined}
										forceClass={ccomponents[component.type].customCss[name].class !== undefined}
										tooltip={ccomponents[component.type].customCss[name].tooltip}
										{name}
										wmClass={getSelector(name)}
										componentType={component.type}
										bind:value={component.customCss[name]}
										on:change={() => {
											console.log('change')
											app.set($app)
										}}
										shouldDisplayRight={hasStyleValue(component.customCss[name])}
										on:right={() => {
											copyLocalToGlobal(name, component?.customCss?.[name])
											tab = 'global'
										}}
										overridding={hasStyleValue($app.css?.[component.type]?.[name]) &&
											hasStyleValue(component.customCss[name])}
									/>
								</div>
							{/each}
						</div>
					{:else}
						<div class="text-sm text-secondary mx-2">No local CSS to display</div>
					{/if}
				</TabContent>
				<TabContent value="global">
					{#if type}
						{#each customCssByComponentType ?? [] as { id, forceStyle, forceClass }}
							<div class="w-full">
								{#if $app.css && type && $app.css[type] && component?.customCss}
									<CssPropertyWrapper
										{forceStyle}
										{forceClass}
										{id}
										bind:property={$app.css[type]}
										on:left={() => {
											copyGlobalToLocal(
												id,
												component?.type ? $app?.css?.[component?.type]?.[id] : undefined
											)
											tab = 'local'
										}}
										overriden={hasStyleValue(component.customCss[id])}
										wmClass={getSelector(id)}
									/>
								{/if}
							</div>
						{/each}
					{/if}
				</TabContent>
			{/snippet}
		</Tabs>

		<ConfirmationModal
			title="Confirm overriding global CSS"
			confirmationText="Override global CSS"
			open={Boolean(overrideGlobalCSS)}
			on:confirmed={() => {
				if (overrideGlobalCSS) {
					overrideGlobalCSS()
					overrideGlobalCSS = undefined
				}

				sendUserToast('Global CSS overridden')
			}}
			on:canceled={() => {
				overrideGlobalCSS = undefined
			}}
		>
			<div class="text-primary pb-2">
				The global CSS for this component already exists. Do you want to override it?
			</div>
		</ConfirmationModal>

		<ConfirmationModal
			title="Confirm overriding local CSS"
			confirmationText="Override local CSS"
			open={Boolean(overrideLocalCSS)}
			on:confirmed={() => {
				if (overrideLocalCSS) {
					overrideLocalCSS()
					overrideLocalCSS = undefined
				}

				sendUserToast('Local CSS overridden')
			}}
			on:canceled={() => {
				overrideLocalCSS = undefined
			}}
		>
			<div class="text-primary pb-2">
				The local CSS for this component already exists. Do you want to override it?
			</div>
		</ConfirmationModal>
		<CssMigrationModal bind:this={migrationModal} bind:component />
	{/key}
{:else}
	<span class="text-sm text-secondary mx-2">Select a component to style it in this panel</span>
{/if}
