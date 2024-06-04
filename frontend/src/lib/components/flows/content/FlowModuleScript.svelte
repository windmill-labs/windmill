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
	let lock: string | undefined = undefined

	let notFound = false
	async function loadCode(path: string, hash: string | undefined) {
		try {
			notFound = false
			const script = hash
				? await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })
				: await getScriptByPath(path!)
			code = script.content
			language = script.language
			lock = script.lock
		} catch (e) {
			notFound = true
			console.error(e)
		}
	}

	$: path && loadCode(path, hash)
</script>

<div class="flex flex-col flex-1 h-full overflow-auto p-2">
	{#if notFound}
		<div class="text-red-400">script not found at {path} in workspace {$workspaceStore}</div>
	{:else}
		<HighlightCode {language} {code} />
	{/if}

	{#if lock}
		<h3 class="mb-2 mt-6">Lock</h3>
		<pre class="bg-surface-secondary text-xs p-2 overflow-auto w-full">{lock}</pre>
	{/if}
</div>
