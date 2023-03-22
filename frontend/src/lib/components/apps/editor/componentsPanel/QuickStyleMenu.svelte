<script lang="ts">
	import { getContext, onMount, setContext } from 'svelte'
	import parse from 'style-to-object'
	import { isValidHexColor } from '../../../../utils'
	import type { AppViewerContext } from '../../types'
	import {
		createStyleStore,
		STYLE_STORE_KEY,
		type StylePropertyKey,
		type TopColors
	} from './quickStyleProperties'
	import QuickStyleProperty from './QuickStyleProperty.svelte'

	export let value = ''
	export let properties: StylePropertyKey[]
	const { app } = getContext<AppViewerContext>('AppViewerContext')
	const styleStore = createStyleStore(properties)
	setContext(STYLE_STORE_KEY, styleStore)
	let mounted = false

	$: mounted && $styleStore && writeStyle()
	$: mounted && !value && parseStyle()
	$: $app && setTopColors()

	function parseStyle() {
		if (!value) {
			styleStore.resetStyle()
			return
		}
		const current = parse(value) || {}
		Object.entries(current).forEach(([k, v]) => {
			styleStore.updateProp(k as StylePropertyKey, v)
		})
		setTopColors()
	}

	function writeStyle() {
		const current = parse(value) || {}
		$styleStore.style.forEach((s) => (current[s.prop.key] = s.value || ''))
		value = Object.entries(current).reduce((style, [k, v]) => {
			return v ? `${style} ${k}: ${v}; `.trim() : style
		}, '')
	}

	function setTopColors() {
		const styles = collectStyles()
		const parsedStyles = styles.map((style) => parse(style) || {})
		const usedColors: Record<string, number> = {}
		parsedStyles.forEach((style) => {
			Object.entries(style).reduce((colors, [k, v]) => {
				// TODO: support RGB and HSL colors as well
				if (isValidHexColor(v)) {
					colors[v] = (colors[v] || 0) + 1
				}
				return colors
			}, usedColors)
		})

		const colors = Object.entries(usedColors)
			.sort((a, b) => b[1] - a[1])
			.slice(0, 3)
			.map(([color]) => color) as TopColors
		styleStore.setTopColors(colors)
	}

	function collectStyles() {
		const styles: string[] = []
		// Getting global app styles
		Object.values($app.css || {}).forEach((element) => {
			Object.values(element).filter(({ style }) => style && styles.push(style))
		})
		// Getting styles from individual components
		$app.grid.map((component) => {
			Object.values(component.data.customCss || {}).forEach(({ style }) => {
				style && styles.push(style)
			})
		})
		// Getting style from subgrids
		Object.values($app.subgrids || {}).forEach((grid) => {
			grid.map((component) => {
				Object.values(component.data.customCss || {}).forEach(({ style }) => {
					style && styles.push(style)
				})
			})
		})
		return styles
	}

	onMount(() => {
		parseStyle()
		mounted = true
	})
</script>

<div class="bg-gray-200/80 rounded-md p-2">
	{#each $styleStore.style as { prop }, index}
		<div class="pb-3 last:pb-0">
			<div class="text-xs font-medium pb-0.5"> {prop.key} </div>
			<div class="flex items-center gap-1 w-full">
				{#if Array.isArray(prop.value)}
					<div class="flex justify-start items-center flex-wrap gap-x-4 gap-y-1">
						{#each prop.value as value}
							<QuickStyleProperty
								prop={{ ...prop, value }}
								inline
								bind:value={$styleStore.style[index].value}
							/>
						{/each}
					</div>
				{:else}
					<QuickStyleProperty {prop} bind:value={$styleStore.style[index].value} />
				{/if}
			</div>
		</div>
	{/each}
</div>
