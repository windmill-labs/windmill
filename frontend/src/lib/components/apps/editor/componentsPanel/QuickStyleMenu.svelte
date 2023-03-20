<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import parse from 'style-to-object'
	import { isValidHexColor } from '../../../../utils'
	import { Button, ClearableInput } from '../../../common'
	import type { AppViewerContext } from '../../types'
	import ColorInput from '../settingsPanel/inputEditor/ColorInput.svelte'

	type TopColors = [] | [string] | [string, string] | [string, string, string]

	export let value = ''
	const { app } = getContext<AppViewerContext>('AppViewerContext')
	const style = {
		'background-color': {
			value: ''
		},
		padding: {
			value: '',
			suggestions: ['2px', '4px', '8px']
		},
		margin: {
			value: '',
			suggestions: ['2px', '4px', '8px']
		},
		'border-radius': {
			value: '',
			suggestions: ['2px', '4px', '8px']
		},
		'border-width': {
			value: '',
			suggestions: ['1px', '2px', '4px']
		},
		'border-color': {
			value: ''
		}
	} as const
	let topColors: TopColors = []
	let mounted = false

	$: mounted && style && writeStyle()
	$: mounted && !value && parseStyle()
	$: $app && setTopColors()

	function parseStyle() {
		if (!value) {
			Object.keys(style).forEach((k) => (style[k].value = ''))
			return
		}
		const current = parse(value) || {}
		Object.entries(current).forEach(([k, v]) => {
			if (k in style) {
				style[k].value = v
			}
		})
		setTopColors()
	}

	function writeStyle() {
		const keys = Object.keys(style) as (keyof typeof style)[]
		const current = parse(value) || {}
		keys.forEach((key) => (current[key] = style[key].value))
		value = Object.entries(current).reduce((style, [k, v]) => {
			return v ? `${style} ${k}: ${v}; `.trim() : style
		}, '')
	}

	function setTopColors() {
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

		const parsedStyles = styles.map((style) => parse(style) || {})
		const usedColors: Record<string, number> = {}
		parsedStyles.forEach((style) => {
			Object.entries(style).reduce((colors, [k, v]) => {
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

	onMount(() => {
		parseStyle()
		mounted = true
	})
</script>

<div class="bg-gray-200/80 rounded-md p-2">
	{#each Object.keys(style) as key}
		<!-- svelte-ignore a11y-label-has-associated-control -->
		<label class="block pb-2">
			<div class="text-xs font-medium pb-0.5"> {key} </div>
			<div class="flex items-center gap-1 w-full">
				{#if key.includes('color')}
					<ColorInput bind:value={style[key].value} on:change={setTopColors} />
					{#each topColors as color}
						<Button
							color="light"
							size="xs"
							variant="border"
							btnClasses="!p-0 !w-[34px] !h-[34px]"
							aria-label="Set {key} to {color}"
							title="Set {key} to {color}"
							style={`background-color: ${color};`}
							on:click={() => (style[key].value = color)}
						/>
					{/each}
				{:else}
					<ClearableInput inputClass="min-w-[32px]" bind:value={style[key].value} />
					{#each style[key]?.suggestions || [] as suggestion}
						<Button
							color="light"
							size="xs"
							variant="border"
							btnClasses="!p-1 !min-w-[34px] !h-[34px]"
							aria-label="Set {key} to {suggestion}"
							title="Set {key} to {suggestion}"
							on:click={() => (style[key].value = suggestion)}
						>
							{suggestion}
						</Button>
					{/each}
				{/if}
			</div>
		</label>
	{/each}
</div>
