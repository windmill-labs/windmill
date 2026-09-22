import { csharp, go, java, json, plaintext, type LanguageType } from 'svelte-highlight/languages'
import graphql from 'svelte-highlight/languages/graphql'
import javascript from 'svelte-highlight/languages/javascript'
import php from 'svelte-highlight/languages/php'
import python from 'svelte-highlight/languages/python'
import powershell from 'svelte-highlight/languages/powershell'
import r from 'svelte-highlight/languages/r'
import ruby from 'svelte-highlight/languages/ruby'
import rust from 'svelte-highlight/languages/rust'
import shell from 'svelte-highlight/languages/shell'
import sql from 'svelte-highlight/languages/sql'
import typescript from 'svelte-highlight/languages/typescript'
import yaml from 'svelte-highlight/languages/yaml'

const LANGUAGE_BY_MONACO: Record<string, LanguageType<string>> = {
	typescript,
	javascript,
	python,
	json,
	yaml,
	sql,
	shell,
	powershell,
	php,
	rust,
	graphql,
	csharp,
	nu: plaintext,
	java,
	r,
	go,
	ruby,
	text: plaintext,
	plaintext,
	bun: typescript,
	bunnative: typescript,
	deno: typescript,
	frontend: typescript,
	nativets: typescript,
	tsx: typescript,
	jsx: javascript,
	python3: python,
	bash: shell,
	ansible: yaml,
	dbt: yaml,
	postgresql: sql,
	mysql: sql,
	bigquery: sql,
	oracledb: sql,
	snowflake: sql,
	mssql: sql,
	duckdb: sql
}

export const TOOL_CODE_DIFF_LANGUAGES = Object.keys(LANGUAGE_BY_MONACO)

export function toolCodeDiffLanguage(language: string): LanguageType<string> {
	return LANGUAGE_BY_MONACO[language] ?? plaintext
}
