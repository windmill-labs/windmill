<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import type { SvelteComponent } from 'svelte'
	import { GoIcon, PythonIcon, TypeScriptIcon } from './'

	export let lang: SupportedLanguage
	export let width = 16
	export let height = 16
	export let scale = 1
	const langToComponent: Record<SupportedLanguage, typeof SvelteComponent> = {
		go: GoIcon,
		python3: PythonIcon,
		deno: TypeScriptIcon
	}
</script>

<svelte:component
	this={langToComponent[lang]}
	width={width * scale}
	height={height * scale}
	{...$$restProps}
/>
