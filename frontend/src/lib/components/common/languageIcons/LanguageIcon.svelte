<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import MySQLIcon from '$lib/components/icons/Mysql.svelte'
	import PostgresIcon from '$lib/components/icons/PostgresIcon.svelte'
	import type { SvelteComponent } from 'svelte'
	import { BashIcon, GoIcon, PythonIcon, TypeScriptIcon } from './'
	import JavaScript from './JavaScript.svelte'
	import FetchIcon from './FetchIcon.svelte'
	import DockerIcon from '$lib/components/icons/DockerIcon.svelte'
	import RestIcon from '$lib/components/icons/RestIcon.svelte'

	export let lang: SupportedLanguage | 'mysql' | 'bun' | 'pgsql' | 'javascript' | 'fetch' | 'docker'
	export let width = 30
	export let height = 30
	export let scale = 1

	const langToComponent: Record<
		SupportedLanguage | 'pgsql' | 'javascript' | 'fetch' | 'docker',
		typeof SvelteComponent
	> = {
		go: GoIcon,
		python3: PythonIcon,
		deno: TypeScriptIcon,
		bun: TypeScriptIcon,
		bash: BashIcon,
		pgsql: PostgresIcon,
		mysql: MySQLIcon,
		javascript: JavaScript,
		fetch: FetchIcon,
		docker: DockerIcon,
		postgresql: PostgresIcon,
		nativets: RestIcon,
		graphql: RestIcon
	}
</script>

<svelte:component
	this={langToComponent[lang]}
	width={width * scale}
	height={height * scale}
	{...$$restProps}
/>
