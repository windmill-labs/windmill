<script lang="ts">
	import { onMount } from 'svelte'
	import parse from 'style-to-object'
	import ColorInput from '../settingsPanel/inputEditor/ColorInput.svelte'

	export let value = ''
	const style = {
		'background-color': '',
		'border-width': '2px',
		'border-color': ''
	} as const
	let mounted = false

	$: mounted && style && writeStyle()

	function parseStyle() {
		if (!value) {
			Object.keys(style).forEach((key) => (style[key] = ''))
			return
		}
		const current = parse(value) || {}
		Object.entries(current).forEach(([k, v]) => {
			if (k in style) {
				style[k] = v
			}
		})
	}

	function writeStyle() {
		console.log('write')
		const keys = Object.keys(style) as (keyof typeof style)[]
		const current = parse(value) || {}
		keys.forEach((key) => (current[key] = style[key]))
		value = Object.entries(current).reduce((style, [k, v]) => {
			return v ? `${style} ${k}: ${v}; `.trim() : style
		}, '')
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
			{#if key.includes('color')}
				<ColorInput bind:value={style[key]} />
			{:else}
				<input type="text" bind:value={style[key]} />
			{/if}
		</label>
	{/each}
</div>
