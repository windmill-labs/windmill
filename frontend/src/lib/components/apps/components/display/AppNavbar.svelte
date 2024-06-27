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
	import Button from '$lib/components/common/button/Button.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'navbarcomponent'> | undefined = undefined
	export let render: boolean
	export let navbarItems: NavbarItem[] = []

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

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
	<div class="flex flex-row w-full items-center border-b py-2 px-4 gap-4">
		<div class="font-semibold">
			{resolvedConfig?.title ?? 'No Title'}
		</div>
		{#each navbarItems ?? [] as navbarItem}
			{#if navbarItem.path || !navbarItem.hidden}
				<Popover notClickable disablePopup={!Boolean(navbarItem.caption)}>
					<svelte:fragment slot="text">{navbarItem.caption}</svelte:fragment>
					<Button
						href={navbarItem.path ? `/apps/get/${navbarItem.path}` : undefined}
						target="_blank"
						color="light"
						size="xs"
						disabled={navbarItem.disabled}
					>
						{navbarItem.label ?? 'No Label'}
					</Button>
				</Popover>
			{/if}
		{/each}
	</div>
{/if}
