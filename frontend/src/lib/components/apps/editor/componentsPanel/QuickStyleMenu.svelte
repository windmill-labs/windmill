<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import parse from 'style-to-object'
	import { isValidHexColor } from '../../../../utils'
	import { Button, ClearableInput } from '../../../common'
	import type { AppViewerContext } from '../../types'
	import ColorInput from '../settingsPanel/inputEditor/ColorInput.svelte'
	import { StyleProperty, StylePropertyType, type StylePropertyKey } from './quickStyleProperties'

	type TopColors = [] | [string] | [string, string] | [string, string, string]

	export let value = ''
	export let properties: StylePropertyKey[]
	const { app } = getContext<AppViewerContext>('AppViewerContext')
	const styleArray = StyleProperty.filter((p) => properties.includes(p.key)).map((p) => ({
		prop: p,
		value: ''
	}))
	// const style = {
	// 	'background-color': {
	// 		value: ''
	// 	},
	// 	padding: {
	// 		value: '',
	// 		suggestions: ['2px', '4px', '8px']
	// 	},
	// 	margin: {
	// 		value: '',
	// 		suggestions: ['2px', '4px', '8px']
	// 	},
	// 	'border-radius': {
	// 		value: '',
	// 		suggestions: ['2px', '4px', '8px']
	// 	},
	// 	'border-width': {
	// 		value: '',
	// 		suggestions: ['1px', '2px', '4px']
	// 	},
	// 	'border-color': {
	// 		value: ''
	// 	}
	// } as const
	let topColors: TopColors = []
	let mounted = false

	$: mounted && styleArray && writeStyle()
	$: mounted && !value && parseStyle()
	$: $app && setTopColors()

	function parseStyle() {
		if (!value) {
			styleArray.forEach((s) => (s.value = ''))
			// Object.keys(style).forEach((k) => (style[k].value = ''))
			return
		}
		const current = parse(value) || {}
		Object.entries(current).forEach(([k, v]) => {
			const styleProp = styleArray.find((s) => s.prop.key === k)
			styleProp && (styleProp.value = v)
			// if (k in style) {
			// 	style[k].value = v
			// }
		})
		setTopColors()
	}

	function writeStyle() {
		// const keys = Object.keys(style) as (keyof typeof style)[]
		const current = parse(value) || {}
		styleArray.forEach((s) => (current[s.prop.key] = s.value))
		// keys.forEach((key) => (current[key] = style[key].value))
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

		topColors = Object.entries(usedColors)
			.sort((a, b) => b[1] - a[1])
			.slice(0, 3)
			.map(([color]) => color) as TopColors
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

	function getProperty(key: StylePropertyKey) {
		return styleArray.find((p) => p.prop.key === key)
	}

	onMount(() => {
		parseStyle()
		mounted = true
	})
</script>

<div class="bg-gray-200/80 rounded-md p-2">
	{#each styleArray as { prop }, i}
		<!-- svelte-ignore a11y-label-has-associated-control -->
		<label class="block pb-2">
			<div class="text-xs font-medium pb-0.5"> {prop.key} </div>
			<div class="flex items-center gap-1 w-full">
				{#if Array.isArray(prop.value)}
					<div>TODO: ARRAY PROP</div>
				{:else if prop.value['type'] === StylePropertyType.color}
					<ColorInput bind:value={styleArray[i].value} on:change={setTopColors} />
					{#each topColors as color}
						<Button
							color="light"
							size="xs"
							variant="border"
							btnClasses="!p-0 !w-[34px] !h-[34px]"
							aria-label="Set {prop.key} to {color}"
							title="Set {prop.key} to {color}"
							style={`background-color: ${color};`}
							on:click={() => (styleArray[i].value = color)}
						/>
					{/each}
				{:else}
					<ClearableInput inputClass="min-w-[32px]" bind:value={styleArray[i].value} />
					{#each prop.value['options'] || [] as option}
						<Button
							color="light"
							size="xs"
							variant="border"
							btnClasses="!p-1 !min-w-[34px] !h-[34px]"
							aria-label="Set {prop.key} to {option.text}"
							title="Set {prop.key} to {option.text}"
							on:click={() => (styleArray[i].value = option.text)}
						>
							{#if typeof option.icon === 'string'}
								{option.icon}
							{:else}
								<svelte:component this={option.icon} size={18} />
							{/if}
						</Button>
					{/each}
				{/if}
			</div>
		</label>
	{/each}
</div>
