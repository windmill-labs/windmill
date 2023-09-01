<script lang="ts">
	import { Tab, TabContent } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { getContext, onMount } from 'svelte'
	import type { AppViewerContext, ComponentCssProperty } from '../../types'
	import { ccomponents, type AppComponent } from '../component'
	import CssProperty from '../componentsPanel/CssProperty.svelte'
	import { quickStyleProperties } from '../componentsPanel/quickStyleProperties'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { premiumStore } from '$lib/stores'
	import { secondaryMenu } from './secondaryMenu'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import CssMigrationModal from './CSSMigrationModal.svelte'
	export let component: AppComponent | undefined

	const { app, cssEditorOpen } = getContext<AppViewerContext>('AppViewerContext')

	let tab: 'local' | 'global' = 'local'

	onMount(() => {
		if ($app.css == undefined) $app.css = {}
		if (component && $app.css[component.type] == undefined && customCssByComponentType) {
			$app.css[component.type] = Object.fromEntries(
				customCssByComponentType.map(({ id }) => [id, {}])
			)
		}
	})

	const customCssByComponentType =
		component?.type && $app.css
			? Object.entries($app.css[component.type] || {}).map(([id, v]) => ({
					id,
					forceStyle: v?.style != undefined,
					forceClass: v?.['class'] != undefined
			  }))
			: undefined

	let overrideGlobalCSS: (() => void) | undefined = undefined
	let overrideLocalCSS: (() => void) | undefined = undefined

	function hasValues(obj: ComponentCssProperty | undefined) {
		if (!obj) return false

		return Object.values(obj).some((v) => v !== '')
	}

	function hasStyleValue(obj: ComponentCssProperty | undefined) {
		if (!obj) return false

		return obj.style !== ''
	}

	function copyLocalToGlobal(name: string, value: ComponentCssProperty | undefined) {
		if (!value) {
			sendUserToast('No local CSS to copy')
		} else {
			const type = component?.type

			if (!type) return

			if (hasValues($app.css?.[type]?.[name])) {
				overrideGlobalCSS = () => {
					$app.css![type]![name] = JSON.parse(JSON.stringify(value))
					app.set($app)
				}
			} else {
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
			if (hasValues(value)) {
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

	let type = component?.type

	let migrationModal: CssMigrationModal | undefined = undefined
</script>

<div class="p-2 flex items-start gap-2 flex-row justify-between">
	<Button
		color="blue"
		size="xs2"
		variant="border"
		on:click={() => {
			secondaryMenu.close()
			$cssEditorOpen = true
		}}
	>
		<div class="flex flex-row gap-1 text-xs items-center">
			Open CSS editor{$premiumStore.premium ? '  (EE only)' : ''}
			<Tooltip light>
				You can also use the App CSS Editor to customise the CSS of all components.
			</Tooltip>
		</div>
	</Button>
	<div class="flex flex-row gap-2 items-center justify-between">
		{#if $premiumStore.premium || true}
			<Button
				color="dark"
				size="xs"
				on:click={() => {
					migrationModal?.open()
				}}
			>
				Migrate to CSS editor
			</Button>
		{/if}
	</div>
</div>

<Tabs bind:selected={tab}>
	<Tab value="local" size="xs">
		<div class="flex flex-row gap-2 items-center">
			ID
			<Badge color="indigo" size="xs">
				{component?.id}
			</Badge>

			<Tooltip light>
				You can customise the CSS and the classes of this component instance. Theses customisations
				will only be applied to this component.
			</Tooltip>
		</div>
	</Tab>
	<Tab value="global" size="xs">
		<div class="flex flex-row gap-2 items-center">
			Global: {type ? ccomponents[type].name : ''}

			<Tooltip light>
				You can customise the CSS and the classes of all components of this type. Theses
				customisations will be applied to all components of this type.
			</Tooltip>
		</div>
	</Tab>
	<svelte:fragment slot="content">
		<TabContent value="local">
			{#if component && component.customCss !== undefined}
				{#each Object.keys(ccomponents[component.type].customCss ?? {}) as name}
					<div class="w-full">
						<CssProperty
							quickStyleProperties={quickStyleProperties?.[component.type]?.[name]}
							forceStyle={ccomponents[component.type].customCss[name].style !== undefined}
							forceClass={ccomponents[component.type].customCss[name].class !== undefined}
							tooltip={ccomponents[component.type].customCss[name].tooltip}
							{name}
							componentType={component.type}
							bind:value={component.customCss[name]}
							on:change={() => app.set($app)}
							shouldDisplayRight={hasValues(component.customCss[name])}
							on:right={() => {
								copyLocalToGlobal(name, component?.customCss?.[name])
								tab = 'global'
							}}
							overridding={hasValues(component.customCss[name])}
						/>
					</div>
				{/each}
			{/if}
		</TabContent>
		<TabContent value="global">
			{#if type}
				{#each customCssByComponentType ?? [] as { id, forceStyle, forceClass }}
					<div class="w-full">
						{#if $app.css && type && $app.css[type]}
							<CssProperty
								{forceStyle}
								{forceClass}
								name={id}
								bind:value={$app.css[type][id]}
								shouldDisplayLeft={hasValues($app.css[type][id])}
								on:left={() => {
									copyGlobalToLocal(
										id,
										component?.type ? $app?.css?.[component?.type]?.[id] : undefined
									)
									tab = 'local'
								}}
								overriden={hasStyleValue(component.customCss[id])}
							/>
						{/if}
					</div>
				{/each}
			{/if}
		</TabContent>
	</svelte:fragment>
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
<CssMigrationModal bind:this={migrationModal} {component} />
