<script lang="ts">
	import Highlight from 'svelte-highlight'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'
	import go from 'svelte-highlight/languages/go'

	export let code: string = ''
	export let language: 'python3' | 'deno' | 'go' | undefined

	function getLang(lang: string | undefined) {
		switch (lang) {
			case 'python3':
				return python
			case 'deno':
				return typescript
			case 'go':
				return go
			default:
				return python
		}
	}

	$: lang = getLang(language)
</script>

<Highlight language={lang} {code} />
