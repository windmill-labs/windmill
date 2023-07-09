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
	import { Script } from '$lib/gen'

	export let lang: SupportedLanguage | 'pgsql' | 'mysql' | 'javascript' | 'fetch' | 'docker'
	export let width = 30
	export let height = 30
	export let scale = 1

	const languageLabel = {
		[Script.language.PYTHON3]: 'Python',
		[Script.language.DENO]: 'TypeScript',
		[Script.language.GO]: 'Go',
		[Script.language.BASH]: 'Bash',
		[Script.language.NATIVETS]: 'HTTP',
		[Script.language.GRAPHQL]: 'HTTP',
		[Script.language.POSTGRESQL]: 'Postgresql'
	}

	const langToComponent: Record<
		SupportedLanguage | 'pgsql' | 'mysql' | 'javascript' | 'fetch' | 'docker',
		typeof SvelteComponent
	> = {
		go: GoIcon,
		python3: PythonIcon,
		deno: TypeScriptIcon,
		graphql: TypeScriptIcon,
		bun: TypeScriptIcon,
		bash: BashIcon,
		pgsql: PostgresIcon,
		mysql: MySQLIcon,
		javascript: JavaScript,
		fetch: FetchIcon,
		docker: DockerIcon,
		postgresql: PostgresIcon,
		nativets: RestIcon
	}
</script>

<svelte:component
	this={langToComponent[lang]}
	title={languageLabel[lang]}
	width={width * scale}
	height={height * scale}
	{...$$restProps}
/>
