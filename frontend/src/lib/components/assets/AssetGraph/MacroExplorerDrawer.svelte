<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Copy, ExternalLink, Loader2, RefreshCw, SquareFunction } from 'lucide-svelte'
	import { resource } from 'runed'
	import {
		invalidateWorkspaceMacros,
		listWorkspaceMacrosCached,
		macroDefinitionSql,
		macroSignature,
		type WorkspaceMacro
	} from '../workspaceMacros'

	let {
		// Called with the library script path when the user clicks "open" on a
		// group header. The pipeline page selects the lib node when it's on the
		// current canvas and falls back to the script page otherwise.
		onOpenLib
	}: { onOpenLib?: (path: string) => void } = $props()

	let open = $state(false)
	let refreshKey = $state(0)
	let filter = $state('')

	export function openDrawer() {
		open = true
	}

	let macrosRes = resource(
		() => (open ? { ws: $workspaceStore, key: refreshKey } : undefined),
		(params) => {
			if (!params?.ws) return Promise.resolve([] as WorkspaceMacro[])
			return listWorkspaceMacrosCached(params.ws)
		}
	)

	let filtered = $derived.by(() => {
		const all = macrosRes.current ?? []
		const q = filter.trim().toLowerCase()
		if (!q) return all
		return all.filter((m) => m.name.includes(q) || m.provider_path.toLowerCase().includes(q))
	})

	// provider_path → its macros, preserving the endpoint's (provider, name) order.
	let byProvider = $derived.by(() => {
		const groups = new Map<string, WorkspaceMacro[]>()
		for (const m of filtered) {
			const g = groups.get(m.provider_path)
			if (g) g.push(m)
			else groups.set(m.provider_path, [m])
		}
		return groups
	})

	function copyCall(m: WorkspaceMacro) {
		navigator.clipboard.writeText(`${m.name}(${m.params})`)
		sendUserToast(`Copied ${m.name}(…) call`)
	}
</script>

<Drawer {open} size="700px" on:close={() => (open = false)}>
	<DrawerContent title="Workspace macros" on:close={() => (open = false)}>
		{#snippet actions()}
			<Button
				unifiedSize="sm"
				variant="subtle"
				startIcon={{ icon: RefreshCw }}
				iconOnly
				title="Refresh"
				onclick={() => {
					if ($workspaceStore) invalidateWorkspaceMacros($workspaceStore)
					refreshKey++
				}}
			/>
		{/snippet}
		<div class="flex flex-col gap-3 h-full">
			<div class="text-xs text-secondary">
				DuckDB macros from deployed <span class="font-mono">// macros</span> libraries. Call them by
				name from any DuckDB script in this workspace — Windmill injects the definitions at run
				time. Add
				<span class="font-mono">// use &lt;library path&gt;</span> to pull in a whole library when the
				call is built dynamically.
			</div>
			<TextInput
				size="md"
				inputProps={{ placeholder: 'Filter by macro name or library path…' }}
				bind:value={filter}
			/>
			{#if macrosRes.loading && !macrosRes.current}
				<div class="flex items-center gap-2 text-tertiary text-xs py-8 justify-center">
					<Loader2 size={14} class="animate-spin" /> Loading macros…
				</div>
			{:else if (macrosRes.current ?? []).length === 0}
				<div class="text-xs text-tertiary py-8 text-center">
					No macro libraries deployed yet. Create one from the pipeline editor's “+” menu (output
					kind “Macro library”), or deploy a DuckDB script whose header starts with
					<span class="font-mono">// macros</span>.
				</div>
			{:else if filtered.length === 0}
				<div class="text-xs text-tertiary py-8 text-center">No macros match the filter.</div>
			{:else}
				<div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4 pr-1">
					{#each [...byProvider.entries()] as [provider, macros] (provider)}
						<div class="flex flex-col gap-1.5">
							<div class="flex items-center gap-2 sticky top-0 bg-surface py-1">
								<SquareFunction size={14} class="shrink-0 text-violet-600 dark:text-violet-400" />
								<span class="text-xs font-mono font-semibold text-emphasis truncate">
									{provider}
								</span>
								<span class="text-2xs text-tertiary">
									{macros.length} macro{macros.length > 1 ? 's' : ''}
								</span>
								{#if onOpenLib}
									<Button
										unifiedSize="sm"
										variant="subtle"
										startIcon={{ icon: ExternalLink }}
										iconOnly
										title="Open this library"
										onclick={() => onOpenLib?.(provider)}
									/>
								{/if}
							</div>
							{#each macros as m (m.name)}
								<div class="flex flex-col gap-1 border rounded-md px-2.5 py-1.5">
									<div class="flex items-center gap-2 min-w-0">
										<span class="flex-1 min-w-0 text-xs font-mono text-emphasis truncate">
											{macroSignature(m)}
										</span>
										<span
											class="shrink-0 text-3xs px-1 py-0.5 rounded-sm bg-surface-secondary text-secondary"
										>
											{m.is_table ? 'table' : 'scalar'}
										</span>
										<Button
											unifiedSize="sm"
											variant="subtle"
											startIcon={{ icon: Copy }}
											iconOnly
											title={`Copy ${m.name}(${m.params})`}
											onclick={() => copyCall(m)}
										/>
									</div>
									<pre
										class="text-2xs font-mono text-secondary whitespace-pre-wrap break-words max-h-32 overflow-y-auto bg-surface-secondary rounded-sm px-2 py-1"
										title={macroDefinitionSql(m)}>{m.body}</pre
									>
								</div>
							{/each}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
