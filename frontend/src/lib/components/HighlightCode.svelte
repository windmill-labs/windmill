<script lang="ts">
	import Highlight, { LineNumbers } from 'svelte-highlight'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'
	import go from 'svelte-highlight/languages/go'
	import shell from 'svelte-highlight/languages/shell'
	import type { Script } from '$lib/gen'

	export let code: string = ''
	export let language: Script.language | undefined
	export let lines = false

	function getLang(lang: string | undefined) {
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
			default:
				return typescript
		}
	}

	$: lang = getLang(language)
</script>

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
