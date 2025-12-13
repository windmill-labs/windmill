<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'

	export type Modules = {
		installed: Record<string, string>
		discovered: {
			direct: string[]
			indirect: string[]
			dev: string[]
		}
	}

	let props = $props<{
		modules: Modules | undefined
	}>()

	let search = $state('')

	const filteredModules = $derived({
		direct: search
			? (props.modules?.discovered?.direct?.filter((mod) =>
					mod.toLowerCase().includes(search.toLowerCase())
				) ?? [])
			: (props.modules?.discovered?.direct ?? []),
		indirect: search
			? (props.modules?.discovered?.indirect?.filter((mod) =>
					mod.toLowerCase().includes(search.toLowerCase())
				) ?? [])
			: (props.modules?.discovered?.indirect ?? []),
		dev: search
			? (props.modules?.discovered?.dev?.filter((mod) =>
					mod.toLowerCase().includes(search.toLowerCase())
				) ?? [])
			: (props.modules?.discovered?.dev ?? [])
	})
</script>

<PanelSection
	size="sm"
	collapsible
	initiallyCollapsed
	titlePadding="pl-1 !text-tertiary"
	fullHeight={false}
	title="packages ({Object.keys(props.modules?.installed ?? {}).length})"
	id="app-editor-frontend-panel-modules"
>
	<input type="text" class="w-full max-w-sm" placeholder="Search packages" bind:value={search} />
	<div class="mt-2 flex flex-col gap-4 w-full">
		{#each ['direct', 'indirect', 'dev'] as type}
			{@const typeModules = filteredModules[type]}
			{#if typeModules.length > 0}
				<div class="flex flex-col gap-1">
					<div class="text-sm px-2 text-secondary font-mono uppercase"
						>{type}
						<span class="text-2xs text-tertiary">({typeModules.length})</span></div
					>
					{#each typeModules as mod (mod)}
						<div class="text-xs px-2 text-secondary font-mono flex justify-between w-full"
							><div class="truncate">{mod}</div>
							<div class="flex items-center gap-1">
								{#if props.modules?.installed?.[mod]}
									<span class="text-2xs px-2 text-tertiary font-mono">
										{props.modules?.installed?.[mod]}
									</span>
								{:else}
									<Loader2 class="w-4 h-4 animate-spin text-tertiary" />
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{/each}
	</div>
</PanelSection>
