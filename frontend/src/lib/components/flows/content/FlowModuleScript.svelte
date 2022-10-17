<script lang="ts">
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import type { FlowModule } from '$lib/gen'
	import { getScriptByPath } from '$lib/utils'

	export let flowModule: FlowModule

	let code: string
	let language: 'deno' | 'python3' | 'go'

	async function loadCode(module: FlowModule) {
		if (module.value.type == 'script') {
			const script = await getScriptByPath(module.value.path!)
			code = script.content
			language = script.language
		} else {
			throw Error('Not a script')
		}
	}

	$: flowModule && loadCode(flowModule)
</script>

<div class="flex flex-col flex-1 h-full overflow-auto p-2">
	<HighlightCode {language} {code} />
</div>
