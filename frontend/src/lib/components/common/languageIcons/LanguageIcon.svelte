<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import MySQLIcon from '$lib/components/icons/Mysql.svelte'
	import PostgresIcon from '$lib/components/icons/PostgresIcon.svelte'
	import type { SvelteComponent } from 'svelte'
	import { BashIcon, GoIcon, PythonIcon, TypeScriptIcon } from './'
	import JavaScript from './JavaScript.svelte'

	export let lang: SupportedLanguage | 'pgsql' | 'mysql' | 'javascript'
	export let width = 30
	export let height = 30
	export let scale = 1
	const langToComponent: Record<
		SupportedLanguage | 'pgsql' | 'mysql' | 'javascript',
		typeof SvelteComponent
	> = {
		go: GoIcon,
		python3: PythonIcon,
		deno: TypeScriptIcon,
		bash: BashIcon,
		pgsql: PostgresIcon,
		mysql: MySQLIcon,
		javascript: JavaScript
	}
</script>

<svelte:component
	this={langToComponent[lang]}
	width={width * scale}
	height={height * scale}
	{...$$restProps}
/>
