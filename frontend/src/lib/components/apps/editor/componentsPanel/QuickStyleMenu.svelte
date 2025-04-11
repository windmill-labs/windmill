<script lang="ts">
	import { getContext, onMount, setContext } from 'svelte'
	import parse from 'style-to-object'
	import { isValidHexColor } from '../../../../utils'
	import type { AppViewerContext } from '../../types'
	import {
		createStyleStore,
		StylePropertyType,
		StylePropertyUnits,
		STYLE_STORE_KEY,
		type PropertyGroup,
		type StylePropertyKey,
		type TopColors
	} from './quickStyleProperties'
	import QuickStyleProperty from './QuickStyleProperty.svelte'
	import ListItem from './ListItem.svelte'
	import type { TypedComponent } from '../component'

	export let value = ''
	export let properties: PropertyGroup[]
	export let componentType: TypedComponent['type'] | undefined = undefined
	export let componentProperty: string | undefined = undefined
	const { app } = getContext<AppViewerContext>('AppViewerContext')
	const styleStore = createStyleStore(properties)
	setContext(STYLE_STORE_KEY, styleStore)

	let multiValues: Record<number, string[]> = initiateMultiValues()
	let mounted = false
	let isOpen: Record<string, boolean> = {}

	$: mounted && $styleStore && writeStyle()
	$: mounted && (!value || value) && parseStyle()
	$: $app && setTopColors()

	function parseStyle() {
		styleStore.resetStyle()
		if (!value) {
			multiValues = initiateMultiValues()
			return
		}
		try {
			const current = parse(value) || {}
			Object.entries(current).forEach(([k, v]) => {
				styleStore.updatePropValue(k as StylePropertyKey, v)
				const { prop, index } = styleStore.getProp(k as StylePropertyKey)
				if (Array.isArray(prop?.prop?.value) && index !== undefined) {
					const valueArray = v.split(' ')
					multiValues[index] = multiValues[index].map((v, i) => valueArray[i] || v)
				}
			})
			setTopColors()
		} catch {}
	}

	function writeStyle() {
		const current = parse(value) || {}
		$styleStore.style.forEach((s) => {
			current[s.prop.key] = convertValue(s.value)
		})
		const entries = Object.entries(current)
		const newValue = entries.reduce((style, [k, v]) => {
			return v ? `${style} ${k}: ${v}; `.trim() : style
		}, '')
		if (value !== newValue) value = newValue
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
		const parsedStyles = styles.map((style) => {
			try {
				return parse(style) || {}
			} catch {
				return {}
			}
		})

		const usedColors: Record<string, number> = {}
		parsedStyles.forEach((style) => {
			Object.values(style).reduce((colors, v) => {
				// TODO: support RGB and HSL colors as well

				// Splitting is needed so colors can be extracted
				// from values like '1px solid #000000'
				v.split(' ').forEach((v) => {
					if (isValidHexColor(v)) {
						colors[v] = (colors[v] || 0) + 1
					}
				})
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

	function initiateMultiValues() {
		return $styleStore.style.reduce((acc, curr, i) => {
			if (Array.isArray(curr.prop.value)) {
				acc[i] = Array.from({ length: curr.prop.value.length }, () => {
					return ''
				})
			}
			return acc
		}, {})
	}

	function setMultiValueProperty(index: number) {
		if (multiValues[index].every((v) => !v || +v === 0 || StylePropertyUnits.includes(v))) {
			$styleStore.style[index].value = ''
			return
		}
		const values = multiValues[index].map((v, i) => {
			v = StylePropertyUnits.includes(v) ? '' : v
			const type = $styleStore.style[index].prop.value[i].type
			if (v) {
				multiValues[index][i] = v
				return v
			} else if (type === StylePropertyType.color) {
				return '#000000'
			} else if (type === StylePropertyType.number) {
				return 0
			} else if (type === StylePropertyType.unit) {
				return 0
			} else if (type === StylePropertyType.text) {
				const options = $styleStore.style[index].prop.value[i].options
				return options ? options[0].text : ''
			} else {
				return ''
			}
		})
		$styleStore.style[index].value = values.join(' ').trim()
	}

	function formatKebabCase(text: string) {
		text = text.toLowerCase().replace(/-/, ' ')
		if (text.length) {
			text = text[0].toUpperCase() + text.slice(1)
		}
		return text
	}

	onMount(() => {
		parseStyle()
		mounted = true
	})
</script>

<div class="flex flex-col !divide-y">
	{#each properties as property}
		{#each Object.keys(property) as group (group)}
			{@const prefix = `${componentType}_${componentProperty}_${group}`}
			<ListItem
				bind:isOpen={isOpen[prefix]}
				title={group}
				{prefix}
				openByDefault={false}
				wrapperClasses="!px-0 !py-0 "
				toggleClasses=" !rounded-b-none !py-0
				{isOpen[prefix] ? '!bg-surface-secondary hover:!bg-surface-hover' : ''}"
			>
				<svelte:fragment slot="title">
					<span class="font-normal text-xs">
						{group}
					</span>
				</svelte:fragment>
				<div class="flex justify-start items-center flex-wrap gap-x-4 gap-y-1 pt-3">
					{#each property[group] as p}
						{@const {
							prop: { prop },
							index
						} = styleStore.getProp(p)}
						{#if prop !== undefined && index !== undefined}
							<div class="pb-2 pt-1">
								<div class="text-xs font-semibold text-tertiary pb-0.5">
									{formatKebabCase(prop.key)}
								</div>
								<div class="flex items-center gap-1">
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
										<QuickStyleProperty {prop} inline bind:value={$styleStore.style[index].value} />
									{/if}
								</div>
							</div>
						{/if}
					{/each}
				</div>
			</ListItem>
		{/each}
	{/each}
</div>
