<script lang="ts">
	import Highlight from 'svelte-highlight'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'
	import go from 'svelte-highlight/languages/go'
	import shell from 'svelte-highlight/languages/shell'

	export let code: string = ''
	export let language: 'python3' | 'deno' | 'go' | 'bash' | undefined

	function getLang(lang: string | undefined) {
		switch (lang) {
			case 'python3':
				return python
			case 'deno':
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

<Highlight className="nowrap {$$props.class}" language={lang} {code} />
