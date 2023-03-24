<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { ccomponents, type AppComponent } from '../component'
	import CssProperty from '../componentsPanel/CssProperty.svelte'
	import { quickStyleProperties } from '../componentsPanel/quickStyleProperties'

	export let component: AppComponent | undefined
	const { app } = getContext<AppViewerContext>('AppViewerContext')
</script>

{#if component && component.customCss !== undefined}
	{#each Object.keys(ccomponents[component.type].customCss ?? {}) as name}
		<div class="w-full">
			<CssProperty
				quickStyleProperties={quickStyleProperties?.[component.type]?.[name]}
				forceStyle={ccomponents[component.type].customCss[name].style !== undefined}
				forceClass={ccomponents[component.type].customCss[name].class !== undefined}
				{name}
				componentType={component.type}
				bind:value={component.customCss[name]}
				on:change={() => app.set($app)}
			/>
		</div>
	{/each}
{/if}
