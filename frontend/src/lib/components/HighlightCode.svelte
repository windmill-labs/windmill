<script lang="ts">
	import Highlight from 'svelte-highlight'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'
	import go from 'svelte-highlight/languages/go'

	export let code: string = ''
	export let language: 'python3' | 'deno' | 'go' | undefined

	function getLang() {
		switch (language) {
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
</script>

<Highlight language={getLang()} {code} />
