<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import MySQLIcon from '$lib/components/icons/Mysql.svelte'
	import PostgresIcon from '$lib/components/icons/PostgresIcon.svelte'
	import { BashIcon, GoIcon, PythonIcon, TypeScriptIcon } from './'
	import JavaScript from './JavaScript.svelte'
	import FetchIcon from './FetchIcon.svelte'
	import DockerIcon from '$lib/components/icons/DockerIcon.svelte'
	import RestIcon from '$lib/components/icons/RestIcon.svelte'
	import PowershellIcon from '$lib/components/icons/PowershellIcon.svelte'
	import BigQueryIcon from '$lib/components/icons/BigQueryIcon.svelte'
	import OracleDBIcon from '$lib/components/icons/OracleDBIcon.svelte'
	import SnowflakeIcon from '$lib/components/icons/SnowflakeIcon.svelte'
	import GraphqlIcon from '$lib/components/icons/GraphqlIcon.svelte'
	import MSSqlServerIcon from '$lib/components/icons/MSSqlServerIcon.svelte'
	import BunIcon from '$lib/components/icons/BunIcon.svelte'
	import DenoIcon from '$lib/components/icons/DenoIcon.svelte'
	import type { Script } from '$lib/gen'
	import PHPIcon from '$lib/components/icons/PHPIcon.svelte'
	import RustIcon from '$lib/components/icons/RustIcon.svelte'
	import AnsibleIcon from '$lib/components/icons/AnsibleIcon.svelte'
	import CSharpIcon from '$lib/components/icons/CSharpIcon.svelte'
	import NuIcon from '$lib/components/icons/NuIcon.svelte'
	import JavaIcon from '$lib/components/icons/JavaIcon.svelte'

	export let lang:
		| SupportedLanguage
		| 'mysql'
		| 'bun'
		| 'pgsql'
		| 'javascript'
		| 'fetch'
		| 'docker'
		| 'powershell'
		| 'bunnative'
	export let width = 30
	export let height = 30
	export let scale = 1

	const languageLabel: Record<Script['language'] | 'bunnative', String> = {
		python3: 'Python',
		deno: 'TypeScript',
		go: 'Go',
		bash: 'Bash',
		powershell: 'PowerShell',
		nativets: 'HTTP',
		bunnative: 'HTTP',
		graphql: 'GraphQL',
		postgresql: 'Postgresql',
		bigquery: 'BigQuery',
		oracledb: 'Oracle Database',
		snowflake: 'Snowflake',
		mysql: 'MySQL',
		mssql: 'MS SQL Server',
		bun: 'TypeScript',
		php: 'PHP',
		rust: 'Rust',
		ansible: 'Ansible Playbook',
		csharp: 'C#',
		nu: 'Nu',
		java: 'Java'
		// for related places search: ADD_NEW_LANG 
	}

	const langToComponent: Record<
		SupportedLanguage | 'pgsql' | 'javascript' | 'fetch' | 'docker' | 'powershell' | 'bunnative',
		any
	> = {
		go: GoIcon,
		python3: PythonIcon,
		deno: TypeScriptIcon,
		// graphql: TypeScriptIcon,
		bun: TypeScriptIcon,
		bunnative: RestIcon,
		bash: BashIcon,
		pgsql: PostgresIcon,
		mysql: MySQLIcon,
		bigquery: BigQueryIcon,
		oracledb: OracleDBIcon,
		snowflake: SnowflakeIcon,
		mssql: MSSqlServerIcon,
		javascript: JavaScript,
		fetch: FetchIcon,
		docker: DockerIcon,
		powershell: PowershellIcon,
		postgresql: PostgresIcon,
		nativets: RestIcon,
		graphql: GraphqlIcon,
		php: PHPIcon,
		rust: RustIcon,
		ansible: AnsibleIcon,
		csharp: CSharpIcon,
		nu: NuIcon,
		java: JavaIcon
		// for related places search: ADD_NEW_LANG 
	}

	let subIconScale = width === 30 ? 0.6 : 0.8
</script>

<div class="relative">
	<svelte:component
		this={langToComponent[lang]}
		title={languageLabel[lang]}
		width={width * scale}
		height={height * scale}
		{...$$restProps}
	/>
	{#if lang === 'deno'}
		<div
			class="absolute -top-1.5 -right-1.5 bg-surface rounded-full flex items-center"
			style={`width: ${width * scale * subIconScale}px; height: ${
				height * scale * subIconScale
			}px;`}
		>
			<DenoIcon width={width * scale * subIconScale} height={height * scale * subIconScale} />
		</div>
	{/if}
	{#if lang === 'bun'}
		<div
			class="absolute -top-1.5 -right-1.5 bg-surface rounded-full flex items-center justify-center"
			style={`width: ${width * scale * subIconScale}px; height: ${
				height * scale * subIconScale
			}px;`}
		>
			<BunIcon
				width={width * scale * (subIconScale - 0.1)}
				height={height * scale * (subIconScale - 0.1)}
			/>
		</div>
	{/if}
</div>
