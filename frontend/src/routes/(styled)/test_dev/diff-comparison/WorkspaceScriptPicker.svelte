<script lang="ts">
	import { ScriptService, type ScriptHistory } from '$lib/gen'
	import Select from '$lib/components/select/Select.svelte'
	import { workspaceStore } from '$lib/stores'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import type { ComparisonCase } from './cases'

	let { onLoaded }: { onLoaded: (c: ComparisonCase) => void } = $props()

	let paths: string[] = $state([])
	let path: string | undefined = $state()
	let history: ScriptHistory[] = $state([])
	let fromHash: string | undefined = $state()
	let toHash: string | undefined = $state()
	let error: string | undefined = $state()

	$effect(() => {
		const workspace = $workspaceStore
		if (!workspace) return
		ScriptService.listScriptPaths({ workspace })
			.then((p) => (paths = p))
			.catch((e) => (error = e.message))
	})

	$effect(() => {
		const workspace = $workspaceStore
		if (!workspace || !path) return
		ScriptService.getScriptHistoryByPath({ workspace, path }).then((h) => {
			history = h
			toHash = h[0]?.script_hash
			fromHash = h[1]?.script_hash ?? h[0]?.script_hash
		})
	})

	$effect(() => {
		const workspace = $workspaceStore
		if (!workspace || !path || !fromHash || !toHash) return
		const p = path
		Promise.all([
			ScriptService.getScriptByHash({ workspace, hash: fromHash }),
			ScriptService.getScriptByHash({ workspace, hash: toHash })
		]).then(([from, to]) => {
			onLoaded({
				id: `workspace:${p}:${from.hash}:${to.hash}`,
				label: p,
				filename: p.split('/').pop() ?? p,
				diff: {
					lang: scriptLangToEditorLang(to.language),
					beforeLang: scriptLangToEditorLang(from.language),
					before: from.content,
					after: to.content
				}
			})
		})
	})

	const versionItems = $derived(
		history.map((h) => ({
			value: h.script_hash,
			label: `${h.script_hash.slice(0, 8)} ${h.deployment_msg ?? ''}`.trim()
		}))
	)
</script>

<div class="flex flex-wrap items-center gap-2 text-xs">
	<span class="text-secondary">Workspace script</span>
	{#if !$workspaceStore}
		<span class="text-hint">Log in and pick a workspace to diff real script versions</span>
	{:else if error}
		<span class="text-red-600">{error}</span>
	{:else}
		<Select
			size="sm"
			placeholder="Script path"
			items={paths.map((p) => ({ value: p, label: p }))}
			bind:value={path}
		/>
		{#if history.length > 0}
			<Select size="sm" items={versionItems} bind:value={fromHash} />
			<span class="text-hint">→</span>
			<Select size="sm" items={versionItems} bind:value={toHash} />
		{/if}
	{/if}
</div>
