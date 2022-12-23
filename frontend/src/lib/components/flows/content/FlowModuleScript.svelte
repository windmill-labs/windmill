<script lang="ts">
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { getScriptByPath } from '$lib/utils'

	export let path: string

	let code: string
	let language: 'deno' | 'python3' | 'go' | 'bash'
	let description: string

	async function loadCode(path: string) {
		const script = await getScriptByPath(path!)
		code = script.content
		language = script.language
	}

	$: path && loadCode(path)
</script>

<div class="flex flex-col flex-1 h-full overflow-auto p-2">
	<HighlightCode {language} {code} />
</div>
