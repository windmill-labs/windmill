<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { ccomponents, type AppComponent } from '../component'
	import CssProperty from '../componentsPanel/CssProperty.svelte'

	export let component: AppComponent | undefined
	const { app } = getContext<AppViewerContext>('AppViewerContext')

	$: if (component?.customCss) {
		app.set($app)
	}
</script>

{#if component}
	{#each Object.keys(ccomponents[component.type].customCss ?? {}) as name}
		{#if component?.customCss !== undefined}
			<div class="w-full">
				<CssProperty
					enableQuickMenu
					forceStyle={ccomponents[component.type].customCss[name].style !== undefined}
					forceClass={ccomponents[component.type].customCss[name].class !== undefined}
					{name}
					bind:value={component.customCss[name]}
				/>
			</div>
		{/if}
	{/each}
{/if}
