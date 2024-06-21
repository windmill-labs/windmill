<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { ScriptService } from '$lib/gen'
	import { getScriptByPath } from '$lib/scripts'
	import { workspaceStore } from '$lib/stores'

	export let path: string
	export let hash: string | undefined = undefined
	export let showDate = false
	export let showAllCode: boolean = true

	let code: string
	let language: SupportedLanguage
	let lock: string | undefined = undefined
	let date: string | undefined = undefined
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
			date = script.created_at
		} catch (e) {
			notFound = true
			console.error(e)
		}
	}

	$: path && loadCode(path, hash)

	function toggleShowAll() {
		showAllCode = !showAllCode
	}
</script>

<div class="flex flex-col flex-1 h-full overflow-auto p-2">
	{#if showDate && date}
		<span class="text-xs text-tertiary mb-4"><TimeAgo {date} /></span>
	{/if}
	{#if notFound}
		<div class="text-red-400">script not found at {path} in workspace {$workspaceStore}</div>
	{:else if showAllCode}
		<HighlightCode {language} {code} />
	{:else}
		<div class="code-container h-full">
			<HighlightCode {language} code={code?.split('\n').slice(0, 10).join('\n')} />
		</div>
		<button on:click={toggleShowAll}>Show all</button>
	{/if}

	{#if lock}
		<h3 class="mb-2 mt-6">Lock</h3>
		<pre class="bg-surface-secondary text-xs p-2 overflow-auto w-full">{lock}</pre>
	{/if}
</div>

<style>
	.code-container {
		position: relative;
		overflow: hidden;
	}
	.code-container::after {
		content: '';
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		height: 100px;
		background: linear-gradient(to bottom, rgba(255, 255, 255, 0), #2e3440);
		pointer-events: none;
	}
</style>
