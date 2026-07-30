<script lang="ts">
	import type { ComponentProps } from 'svelte'
	// A dbt script's modules ARE its dbt project, so they are browsed as the tree
	// dbt itself expects rather than as the flat tab strip a couple of helper
	// files get. Read-only on purpose: dbt development is a local loop (`dbt run
	// --select`, `dbt test` against a dev target) and a browser textarea over one
	// file of a project is a worse version of it.
	import type { ScriptModule } from '$lib/gen'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { ChevronDown, ChevronRight, FileText } from 'lucide-svelte'
	import { Button } from '$lib/components/common'

	let { modules, scriptPath }: { modules: Record<string, ScriptModule>; scriptPath: string } =
		$props()

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
	let fileCount = $derived(Object.keys(modules).length)
	let selected = $state<string | undefined>(undefined)
	let collapsed = $state<Record<string, boolean>>({})
	// `dbt_project.yml` is the file that makes the bundle a project, so it is what
	// opens first; falling back to whatever sorts first keeps a partial bundle
	// from showing an empty pane.
	let effective = $derived(
		selected && modules[selected]
			? selected
			: modules['dbt_project.yml']
				? 'dbt_project.yml'
				: Object.keys(modules).sort()[0]
	)

	// A dbt project holds SQL, YAML and the odd Python model; `ansible` is how
	// this component spells YAML. Anything else (`.md`, `.csv`, `.txt`) falls to
	// `bash`, whose grammar leaves prose alone — NOT `undefined`, which
	// HighlightCode resolves to TypeScript and which would litter a seed file
	// with keyword colouring.
	function langOf(path: string): ComponentProps<typeof HighlightCode>['language'] {
		if (path.endsWith('.sql')) return 'sql'
		if (path.endsWith('.yml') || path.endsWith('.yaml')) return 'ansible'
		if (path.endsWith('.py')) return 'python3'
		return 'bash'
	}
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
			<button
				class="w-full flex items-center gap-1 px-1 py-0.5 text-2xs hover:bg-surface-hover {effective ===
				node.path
					? 'bg-surface-selected font-semibold text-primary'
					: 'text-secondary'}"
				style="padding-left: {depth * 10 + 15}px"
				onclick={() => (selected = node.path)}
			>
				<FileText size={11} class="shrink-0 opacity-60" />
				<span class="truncate font-mono">{node.name}</span>
			</button>
		{/if}
	{/each}
{/snippet}

<div class="h-full flex flex-col min-h-0">
	<div
		class="shrink-0 flex items-center gap-2 px-2 py-1 text-2xs border-b bg-surface-secondary text-secondary"
	>
		<span class="font-mono truncate">{scriptPath}__dbt/</span>
		<span class="opacity-70 shrink-0">{fileCount} file{fileCount === 1 ? '' : 's'}</span>
		<span class="ml-auto shrink-0 opacity-70">read-only · edit locally</span>
	</div>
	{#if fileCount === 0}
		<div class="p-3 text-xs text-secondary">
			This dbt script carries no project yet. Copy one into its <span class="font-mono"
				>{scriptPath}__dbt/</span
			>
			folder and push it:
			<pre class="mt-2 p-2 rounded bg-surface-secondary text-2xs overflow-x-auto"
				>cp -r my-dbt-project/. {scriptPath}__dbt/
wmill sync push</pre
			>
		</div>
	{:else}
		<div class="flex-1 min-h-0 flex">
			<div class="w-56 shrink-0 border-r overflow-auto py-1">
				{@render branch(tree, 0)}
			</div>
			<div class="flex-1 min-w-0 flex flex-col">
				<div
					class="shrink-0 flex items-center gap-2 px-2 py-1 text-2xs border-b text-secondary bg-surface"
				>
					<span class="font-mono truncate">{effective}</span>
					<div class="ml-auto shrink-0">
						<Button
							size="xs2"
							variant="subtle"
							on:click={() => navigator.clipboard.writeText(modules[effective]?.content ?? '')}
						>
							Copy
						</Button>
					</div>
				</div>
				<div class="flex-1 min-h-0 overflow-auto">
					<HighlightCode language={langOf(effective)} code={modules[effective]?.content ?? ''} />
				</div>
			</div>
		</div>
	{/if}
</div>
