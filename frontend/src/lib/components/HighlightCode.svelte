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
	import type { Script } from '$lib/gen'
	import { Button } from './common'
	import { copyToClipboard } from '$lib/utils'
	import { ClipboardCopy } from 'lucide-svelte'

	export let code: string = ''
	export let language: Script.language | 'frontend' | undefined
	export let lines = false

	function getLang(lang: Script.language | 'frontend' | undefined) {
		switch (lang) {
			case 'python3':
				return python
			case 'deno':
				return typescript
			case 'nativets':
				return typescript
			case 'bun':
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
			case 'powershell':
				return powershell

			default:
				return typescript
		}
	}

	$: lang = getLang(language)
</script>

<div class="relative overflow-x-auto">
	<Button
		class="absolute top-2 right-2"
		on:click={() => copyToClipboard(code)}
		color="light"
		size="xs"
	>
		<ClipboardCopy size={12} />
	</Button>
	{#if code?.length < 5000}
		{#if !lines}
			<Highlight class="nowrap {$$props.class}" language={lang} {code} />
		{:else}
			<Highlight class="nowrap {$$props.class}" language={lang} {code} let:highlighted>
				<LineNumbers {highlighted} />
			</Highlight>
		{/if}
	{:else}
		<pre class="overflow-auto max-h-screen {$$props.class}"
			><code class="language-{language}">{code}</code></pre
		>
	{/if}
</div>
