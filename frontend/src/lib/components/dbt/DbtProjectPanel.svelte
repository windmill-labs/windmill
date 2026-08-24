<script lang="ts">
	// A dbt script's modules ARE its project, so they are browsed as the tree dbt
	// expects rather than the flat tab strip helper files get. Selecting one opens
	// it in the editor: the descriptor and the project's files are edited in the
	// same place, because once the descriptor moved inside `__dbt/` there was no
	// longer a boundary between them to draw.
	import type { ScriptModule } from '$lib/gen'
	import type { Snippet } from 'svelte'
	import { ChevronDown, ChevronRight, FileText, Trash2 } from 'lucide-svelte'
	import { dbtProjectFileKey } from './projectFiles'

	let {
		modules,
		scriptPath,
		descriptorName,
		/** `null` is the descriptor, matching the editor's `activeModuleTab`. */
		selected,
		onSelect,
		onDelete,
		/** The add-file control. Owned by the editor, which holds the form. */
		addFile
	}: {
		modules: Record<string, ScriptModule>
		scriptPath: string
		descriptorName: string
		selected: string | null
		onSelect: (path: string | null) => void
		onDelete?: (path: string) => void
		addFile?: Snippet
	} = $props()

	type Node = { name: string; path: string; children: Node[] }

	function buildTree(paths: string[]): Node[] {
		const root: Node = { name: '', path: '', children: [] }
		for (const p of [...paths].sort()) {
			let node = root
			const parts = p.split('/')
			parts.forEach((part, i) => {
				const path = parts.slice(0, i + 1).join('/')
				let next = node.children.find((c) => c.name === part)
				if (!next) {
					// Directories carry their full path too: it is what makes a node
					// identity, and two of the same name at one depth under different
					// parents — `models/staging` and `tests/staging` — would otherwise
					// share a collapse key and fold together.
					next = { name: part, path, children: [] }
					node.children.push(next)
				}
				node = next
			})
		}
		// Directories before files, each alphabetical: the same order `ls` gives,
		// so the tree reads like the checkout on disk.
		const sort = (n: Node) => {
			n.children.sort((a, b) => {
				const aDir = a.children.length > 0
				const bDir = b.children.length > 0
				if (aDir !== bDir) return aDir ? -1 : 1
				return a.name.localeCompare(b.name)
			})
			n.children.forEach(sort)
		}
		sort(root)
		return root.children
	}

	let tree = $derived(buildTree(Object.keys(modules)))
	// The key the bundle actually holds it under, which a project imported with a
	// redundant spelling states differently from the canonical name.
	let projectFileKey = $derived(dbtProjectFileKey(modules))
	let fileCount = $derived(Object.keys(modules).length)
	let collapsed = $state<Record<string, boolean>>({})
</script>

{#snippet branch(nodes: Node[], depth: number)}
	{#each nodes as node (node.name + node.path + depth)}
		{@const key = `${depth}:${node.name}:${node.path}`}
		{#if node.children.length > 0}
			<button
				class="w-full flex items-center gap-1 px-1 py-0.5 text-2xs hover:bg-surface-hover text-secondary"
				style="padding-left: {depth * 10 + 4}px"
				onclick={() => (collapsed[key] = !collapsed[key])}
			>
				{#if collapsed[key]}
					<ChevronRight size={11} class="shrink-0" />
				{:else}
					<ChevronDown size={11} class="shrink-0" />
				{/if}
				<span class="truncate font-mono">{node.name}</span>
			</button>
			{#if !collapsed[key]}
				{@render branch(node.children, depth + 1)}
			{/if}
		{:else}
			<div
				class="group w-full flex items-center hover:bg-surface-hover {selected === node.path
					? 'bg-surface-selected'
					: ''}"
			>
				<button
					class="flex-1 min-w-0 flex items-center gap-1 px-1 py-0.5 text-2xs {selected === node.path
						? 'font-semibold text-primary'
						: 'text-secondary'}"
					style="padding-left: {depth * 10 + 15}px"
					onclick={() => onSelect(node.path)}
				>
					<FileText size={11} class="shrink-0 opacity-60" />
					<span class="truncate font-mono">{node.name}</span>
				</button>
				<!-- `dbt_project.yml` is what makes the bundle a project: without it the
				     worker refuses the version outright, so one click here would deploy a
				     script that cannot run. -->
				{#if onDelete && node.path !== projectFileKey}
					<button
						class="shrink-0 px-1 opacity-0 group-hover:opacity-100 text-secondary hover:text-red-500"
						title="Delete {node.path}"
						onclick={() => onDelete?.(node.path)}
					>
						<Trash2 size={11} />
					</button>
				{/if}
			</div>
		{/if}
	{/each}
{/snippet}

<div class="w-56 shrink-0 border-r flex flex-col min-h-0 bg-surface">
	<div
		class="shrink-0 flex items-center gap-2 px-2 py-1 text-2xs border-b bg-surface-secondary text-secondary"
	>
		<span class="font-mono truncate">{scriptPath}__dbt/</span>
		<div class="ml-auto shrink-0 flex items-center gap-1">
			<span class="opacity-70">{fileCount + 1}</span>
			{@render addFile?.()}
		</div>
	</div>
	<div class="flex-1 min-h-0 overflow-auto py-1">
		<!-- The descriptor sits at the project root, which is where it lives on disk. -->
		<button
			class="w-full flex items-center gap-1 px-1 py-0.5 text-2xs hover:bg-surface-hover {selected ===
			null
				? 'bg-surface-selected font-semibold text-primary'
				: 'text-secondary'}"
			style="padding-left: 15px"
			onclick={() => onSelect(null)}
		>
			<FileText size={11} class="shrink-0 opacity-60" />
			<span class="truncate font-mono">{descriptorName}</span>
		</button>
		{@render branch(tree, 0)}
	</div>
	{#if fileCount === 0}
		<div class="shrink-0 p-2 text-2xs text-secondary border-t">
			No project yet. Copy one in and push it:
			<pre class="mt-1 p-1 rounded bg-surface-secondary overflow-x-auto"
				>cp -r my-dbt-project/. {scriptPath}__dbt/
wmill sync push</pre
			>
		</div>
	{/if}
</div>
