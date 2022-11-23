<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import PostgresIcon from '$lib/components/icons/PostgresIcon.svelte'
	import type { SvelteComponent } from 'svelte'
	import { BashIcon, GoIcon, PythonIcon, TypeScriptIcon } from './'

	export let lang: SupportedLanguage | 'pgsql'
	export let width = 30
	export let height = 30
	export let scale = 1
	const langToComponent: Record<SupportedLanguage | 'pgsql', typeof SvelteComponent> = {
		go: GoIcon,
		python3: PythonIcon,
		deno: TypeScriptIcon,
		bash: BashIcon,
		pgsql: PostgresIcon
	}
</script>

<svelte:component
	this={langToComponent[lang]}
	width={width * scale}
	height={height * scale}
	{...$$restProps}
/>
