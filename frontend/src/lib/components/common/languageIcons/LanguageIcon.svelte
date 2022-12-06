<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import MySQLIcon from '$lib/components/icons/Mysql.svelte'
	import PostgresIcon from '$lib/components/icons/PostgresIcon.svelte'
	import type { SvelteComponent } from 'svelte'
	import { BashIcon, GoIcon, PythonIcon, TypeScriptIcon } from './'

	export let lang: SupportedLanguage | 'pgsql' | 'mysql'
	export let width = 30
	export let height = 30
	export let scale = 1
	const langToComponent: Record<SupportedLanguage | 'pgsql' | 'mysql', typeof SvelteComponent> = {
		go: GoIcon,
		python3: PythonIcon,
		deno: TypeScriptIcon,
		bash: BashIcon,
		pgsql: PostgresIcon,
		mysql: MySQLIcon
	}
</script>

<svelte:component
	this={langToComponent[lang]}
	width={width * scale}
	height={height * scale}
	{...$$restProps}
/>
