<script lang="ts" module>
	let cachedValues: Record<
		string,
		{
			code: string | undefined
			previousCode: string | undefined
			language: ScriptLang | undefined
			lock: string | undefined
			date: string | undefined
			notFound: boolean
			tag: string | undefined
		}
	> = {}
</script>

<script lang="ts">
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { ScriptService, type ScriptLang } from '$lib/gen'
	import { getScriptByPath, scriptLangToEditorLang } from '$lib/scripts'
	import { workspaceStore } from '$lib/stores'
	import { Loader2 } from 'lucide-svelte'
	import { untrack } from 'svelte'

	interface Props {
		path: string
		hash?: string | undefined
		previousHash?: string | undefined
		showDate?: boolean
		showAllCode?: boolean
		tag?: string | undefined
		language?: ScriptLang | undefined
		showDiff?: boolean
	}

	let {
		path,
		hash = undefined,
		previousHash = undefined,
		showDate = false,
		showAllCode = $bindable(true),
		tag = $bindable(undefined),
		showDiff = false,
		language = $bindable(undefined)
	}: Props = $props()

	let code: string | undefined = $state()
	let previousCode: string | undefined = $state()
	let lock: string | undefined = $state(undefined)
	let date: string | undefined = $state(undefined)
	let notFound = $state(false)

	function getCachedKey(path: string, hash: string | undefined) {
		return `${$workspaceStore}-${path}-${hash ?? ''}`
	}
	function getCachedValues(path: string, hash: string | undefined) {
		const key = getCachedKey(path, hash)
		code = cachedValues[key]?.code
		language = cachedValues[key]?.language
		lock = cachedValues[key]?.lock
		date = cachedValues[key]?.date
		previousCode = cachedValues[key]?.previousCode
		tag = cachedValues[key]?.tag
		notFound = cachedValues[key]?.notFound ?? false
	}

	getCachedValues(path, hash)

	async function loadPreviousCode(previousHash: string) {
		try {
			const previousScript = await ScriptService.getScriptByHash({
				workspace: $workspaceStore!,
				hash: previousHash
			})
			previousCode = previousScript.content
			const key = getCachedKey(path, previousHash)
			cachedValues[key] = {
				...(cachedValues[key] ?? {}),
				previousCode: previousScript.content
			}
		} catch (e) {
			console.error(e)
		}
	}

	function toggleShowAll() {
		showAllCode = !showAllCode
	}

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
			tag = script.tag
			const key = getCachedKey(path, hash)
			cachedValues[key] = {
				...(cachedValues[key] ?? {}),
				code: script.content,
				language: script.language,
				lock: script.lock,
				date: script.created_at,
				tag: script.tag
			}
		} catch (e) {
			notFound = true
			console.error(e)
		}
	}

	$effect.pre(() => {
		hash
		path && untrack(() => loadCode(path, hash))
	})
	$effect.pre(() => {
		path && previousHash && untrack(() => loadPreviousCode(previousHash))
	})
</script>

<div class="flex flex-col flex-1 h-full overflow-auto p-2">
	{#if showDate && date}
		<span class="text-xs text-tertiary mb-4"><TimeAgo agoOnlyIfRecent {date} /></span>
	{/if}
	{#if tag}
		<div class="text-xs text-tertiary mb-4">tag: {tag}</div>
	{/if}
	{#if notFound}
		<div class="text-red-400">script not found at {path} in workspace {$workspaceStore}</div>
	{:else if showAllCode}
		{#if showDiff}
			{#key (previousCode ?? '') + (code ?? '')}
				{#await import('$lib/components/DiffEditor.svelte')}
					<Loader2 class="animate-spin" />
				{:then Module}
					<Module.default
						open={true}
						class="h-screen"
						readOnly
						automaticLayout
						defaultLang={scriptLangToEditorLang(language)}
						defaultOriginal={previousCode}
						defaultModified={code}
					/>
				{/await}
			{/key}
		{:else}
			<HighlightCode {language} {code} />
		{/if}
	{:else}
		<div class="code-container h-full">
			<HighlightCode {language} code={code?.split('\n').slice(0, 10).join('\n')} />
		</div>
		<button onclick={toggleShowAll}>Show all</button>
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
		background: linear-gradient(to bottom, rgba(255, 255, 255, 0), rgb(var(--color-surface)));
		pointer-events: none;
	}
</style>
