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

	const fail = (e: unknown) => (error = e instanceof Error ? e.message : String(e))

	$effect(() => {
		const workspace = $workspaceStore
		if (!workspace) return
		ScriptService.listScriptPaths({ workspace })
			.then((p) => (paths = p))
			.catch(fail)
	})

	function selectPath(p: string | undefined) {
		path = p
		history = []
		fromHash = undefined
		toHash = undefined
		const workspace = $workspaceStore
		if (!workspace || !p) return
		ScriptService.getScriptHistoryByPath({ workspace, path: p })
			.then((h) => {
				if (path !== p) return
				history = h
				toHash = h[0]?.script_hash
				fromHash = h[1]?.script_hash ?? h[0]?.script_hash
			})
			.catch(fail)
	}

	$effect(() => {
		const workspace = $workspaceStore
		const [p, from, to] = [path, fromHash, toHash]
		if (!workspace || !p || !from || !to) return
		Promise.all([
			ScriptService.getScriptByHash({ workspace, hash: from }),
			ScriptService.getScriptByHash({ workspace, hash: to })
		])
			.then(([before, after]) => {
				// A later selection may have resolved first.
				if (path !== p || fromHash !== from || toHash !== to) return
				onLoaded({
					id: `workspace:${p}:${from}:${to}`,
					label: p,
					filename: p.split('/').pop() ?? p,
					diff: {
						lang: scriptLangToEditorLang(after.language),
						beforeLang: scriptLangToEditorLang(before.language),
						before: before.content,
						after: after.content
					}
				})
			})
			.catch(fail)
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
	{:else}
		<Select
			size="sm"
			placeholder="Script path"
			items={paths.map((p) => ({ value: p, label: p }))}
			bind:value={() => path, selectPath}
		/>
		{#if history.length > 0}
			<Select size="sm" items={versionItems} bind:value={fromHash} />
			<span class="text-hint">→</span>
			<Select size="sm" items={versionItems} bind:value={toHash} />
		{/if}
		{#if error}
			<span class="text-red-600">{error}</span>
		{/if}
	{/if}
</div>
