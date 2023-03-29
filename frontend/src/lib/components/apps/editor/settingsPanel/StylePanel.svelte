<script lang="ts">
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/utils'
	import { Copy } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { ccomponents, type AppComponent } from '../component'
	import CssProperty from '../componentsPanel/CssProperty.svelte'
	import { quickStyleProperties } from '../componentsPanel/quickStyleProperties'

	export let component: AppComponent | undefined
	const { app } = getContext<AppViewerContext>('AppViewerContext')

	function applyToAllInstances() {
		if (component) {
			if (!$app.css) {
				$app.css = {}
			}
			let componentType = component?.type
			if (!$app.css![componentType]) {
				$app.css![componentType] = {}
			}
			Object.keys(ccomponents[component.type].customCss ?? {}).forEach((name) => {
				if (!$app.css![componentType]![name]) {
					$app.css![componentType]![name] = {}
				}
				if (component) {
					let nstyle = component.customCss![name]
					if (nstyle.style) {
						$app.css![componentType]![name].style = nstyle.style
					}
					if (nstyle.class) {
						$app.css![componentType]![name].class = nstyle.class
					}
				}
			})

			sendUserToast(
				`Applied style to all instances of the ${componentType.replace('component', '')} component`
			)
		}
	}
</script>

<Button
	variant="border"
	color="light"
	size="xs"
	aria-label="Apply to all instances of this component"
	btnClasses="ml-3 mt-2"
	on:click={applyToAllInstances}
>
	Copy style to global CSS &nbsp;<Copy size={18} />
</Button>

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
