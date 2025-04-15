<script lang="ts">
	import Highlight, { LineNumbers } from 'svelte-highlight'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'
	import go from 'svelte-highlight/languages/go'
	import shell from 'svelte-highlight/languages/shell'
	import graphql from 'svelte-highlight/languages/graphql'
	import javascript from 'svelte-highlight/languages/javascript'
	import sql from 'svelte-highlight/languages/sql'
	import powershell from 'svelte-highlight/languages/powershell'
	import php from 'svelte-highlight/languages/php'
	import rust from 'svelte-highlight/languages/rust'
	import csharp from 'svelte-highlight/languages/csharp'
	import yaml from 'svelte-highlight/languages/yaml'
	import java from 'svelte-highlight/languages/java'
	import type { Script } from '$lib/gen'
	import { Button } from './common'
	import { copyToClipboard } from '$lib/utils'
	import { ClipboardCopy } from 'lucide-svelte'
	import HighlightTheme from './HighlightTheme.svelte'
	import type { LanguageType } from 'svelte-highlight/languages'

	export let code: string = ''
	export let language: Script['language'] | 'bunnative' | 'frontend' | undefined
	export let highlightLanguage: LanguageType<string> | undefined = undefined
	export let lines = false

	function getLang(lang: Script['language'] | 'bunnative' | 'frontend' | undefined) {
		switch (lang) {
			case 'python3':
				return python
			case 'deno':
				return typescript
			case 'nativets':
				return typescript
			case 'bun':
				return typescript
			case 'bunnative':
				return typescript
			case 'go':
				return go
			case 'bash':
				return shell
			case 'frontend':
				return javascript
			case 'graphql':
				return graphql
			case 'mysql':
				return sql
			case 'postgresql':
				return sql
			case 'snowflake':
				return sql
			case 'bigquery':
				return sql
			case 'oracledb':
				return sql
			case 'powershell':
				return powershell
			case 'php':
				return php
			case 'rust':
				return rust
			case 'csharp':
				return csharp
			case 'nu':
				return python
			case 'ansible':
				return yaml;
			case 'java':
				return java;
			// for related places search: ADD_NEW_LANG 
			default:
				return typescript
		}
	}

	$: lang = highlightLanguage ?? getLang(language)
</script>

<HighlightTheme />

<div class="relative">
	<Button
		wrapperClasses="absolute top-2 right-2 z-20"
		on:click={() => copyToClipboard(code)}
		color="light"
		size="xs2"
		startIcon={{
			icon: ClipboardCopy
		}}
		iconOnly
	/>
	<div class="overflow-x-auto">
		{#if code?.length < 10000}
			{#if !lines}
				<Highlight class="nowrap {$$props.class}" language={lang} {code} />
			{:else}
				<Highlight class="nowrap {$$props.class}" language={lang} {code} let:highlighted>
					<LineNumbers {highlighted} />
				</Highlight>
			{/if}
		{:else}
			<pre class="overflow-auto max-h-screen text-xs {$$props.class}"
				><code class="language-{language}">{code}</code></pre
			>
		{/if}
	</div>
</div>
