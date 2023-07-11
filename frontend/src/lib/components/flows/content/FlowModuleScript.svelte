<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { ScriptService } from '$lib/gen'
	import { getScriptByPath } from '$lib/scripts'
	import { workspaceStore } from '$lib/stores'

	export let path: string
	export let hash: string | undefined = undefined

	let code: string
	let language: SupportedLanguage

	async function loadCode(path: string, hash: string | undefined) {
		const script = hash
			? await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })
			: await getScriptByPath(path!)
		code = script.content
		language = script.language
	}

	$: path && loadCode(path, hash)
</script>

<div class="flex flex-col flex-1 h-full overflow-auto p-2">
	<HighlightCode {language} {code} />
</div>
