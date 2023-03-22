<script lang="ts">
	import { getContext, onMount, setContext } from 'svelte'
	import parse from 'style-to-object'
	import { isValidHexColor } from '../../../../utils'
	import type { AppViewerContext } from '../../types'
	import {
		createStyleStore,
		StylePropertyType,
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
	const multiValues: Record<number, string[]> = $styleStore.style.reduce((prev, curr, i) => {
		if (Array.isArray(curr.prop.value)) {
			prev[i] = Array.from({ length: curr.prop.value.length }, () => {
				return ''
			})
		}
		return prev
	}, {})
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
		$styleStore.style.forEach((s) => {
			current[s.prop.key] = convertValue(s.value)
		})
		value = Object.entries(current).reduce((style, [k, v]) => {
			return v ? `${style} ${k}: ${v}; `.trim() : style
		}, '')
	}

	function convertValue(value: any) {
		switch (typeof value) {
			case 'number':
				return value + ''
			case 'string':
				return value
			default:
				return ''
		}
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

	function setMultiValueProperty(index: number) {
		if (multiValues[index].every((v) => !v)) {
			$styleStore.style[index].value = ''
			return
		}
		const values = multiValues[index].map((v, i) => {
			const type = $styleStore.style[index].prop.value[i].type
			if (v) {
				return v
			} else if (type === StylePropertyType.color) {
				return '#000000'
			} else if (type === StylePropertyType.number) {
				return '0'
			} else if (type === StylePropertyType.unit) {
				return '0'
			} else if (type === StylePropertyType.text) {
				const options = $styleStore.style[index].prop.value[i].options
				return options ? options[0].text : ''
			} else {
				return ''
			}
		})
		$styleStore.style[index].value = values.join(' ').trim()
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
						{#each prop.value as value, i}
							<QuickStyleProperty
								prop={{ ...prop, value }}
								inline
								bind:value={multiValues[index][i]}
								on:change={() => setMultiValueProperty(index)}
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
