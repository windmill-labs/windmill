<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import MySQLIcon from '$lib/components/icons/Mysql.svelte'
	import PostgresIcon from '$lib/components/icons/PostgresIcon.svelte'
	import { BashIcon, GoIcon, PythonIcon, TypeScriptIcon } from './'
	import JavaScript from './JavaScript.svelte'
	import FetchIcon from './FetchIcon.svelte'
	import DockerIcon from '$lib/components/icons/DockerIcon.svelte'
	import RestIcon from '$lib/components/icons/RestIcon.svelte'
	import { Script } from '$lib/gen'
	import PowershellIcon from '$lib/components/icons/PowershellIcon.svelte'
	import BigQueryIcon from '$lib/components/icons/BigQueryIcon.svelte'
	import SnowflakeIcon from '$lib/components/icons/SnowflakeIcon.svelte'
	import GraphqlIcon from '$lib/components/icons/GraphqlIcon.svelte'
	import MSSqlServerIcon from '$lib/components/icons/MSSqlServerIcon.svelte'

	export let lang:
		| SupportedLanguage
		| 'mysql'
		| 'bun'
		| 'pgsql'
		| 'javascript'
		| 'fetch'
		| 'docker'
		| 'powershell'
	export let width = 30
	export let height = 30
	export let scale = 1

	const languageLabel = {
		[Script.language.PYTHON3]: 'Python',
		[Script.language.DENO]: 'TypeScript',
		[Script.language.GO]: 'Go',
		[Script.language.BASH]: 'Bash',
		[Script.language.POWERSHELL]: 'PowerShell',
		[Script.language.NATIVETS]: 'HTTP',
		[Script.language.GRAPHQL]: 'GraphQL',
		[Script.language.POSTGRESQL]: 'Postgresql',
		[Script.language.BIGQUERY]: 'BigQuery',
		[Script.language.SNOWFLAKE]: 'Snowflake',
		[Script.language.MSSQL]: 'MS SQL Server'
	}

	const langToComponent: Record<
		SupportedLanguage | 'pgsql' | 'javascript' | 'fetch' | 'docker' | 'powershell',
		any
	> = {
		go: GoIcon,
		python3: PythonIcon,
		deno: TypeScriptIcon,
		// graphql: TypeScriptIcon,
		bun: TypeScriptIcon,
		bash: BashIcon,
		pgsql: PostgresIcon,
		mysql: MySQLIcon,
		bigquery: BigQueryIcon,
		snowflake: SnowflakeIcon,
		mssql: MSSqlServerIcon,
		javascript: JavaScript,
		fetch: FetchIcon,
		docker: DockerIcon,
		powershell: PowershellIcon,
		postgresql: PostgresIcon,
		nativets: RestIcon,
		graphql: GraphqlIcon
	}
</script>

<svelte:component
	this={langToComponent[lang]}
	title={languageLabel[lang]}
	width={width * scale}
	height={height * scale}
	{...$$restProps}
/>
