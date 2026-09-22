<script lang="ts">
	import {
		BookOpen,
		Code2,
		Database,
		DollarSign,
		ExternalLink,
		LayoutDashboard,
		Plug,
		ScrollText
	} from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { ACCESS_ENTRY_KIND_ORDER, type AccessEntryKind, type AccessScope } from './folderAccess'

	let { scope, dense = false }: { scope: AccessScope; dense?: boolean } = $props()

	const sectionLabels: Record<AccessEntryKind, string> = {
		mcp: 'MCP servers',
		instruction: 'Instructions',
		skill: 'Skills',
		script: 'Scripts',
		flow: 'Flows',
		app: 'Apps',
		resource: 'Resources',
		variable: 'Variables'
	}
	const kindIcons: Record<AccessEntryKind, typeof BookOpen | typeof BarsStaggered> = {
		mcp: Plug,
		instruction: ScrollText,
		skill: BookOpen,
		script: Code2,
		flow: BarsStaggered,
		app: LayoutDashboard,
		resource: Database,
		variable: DollarSign
	}
</script>

<div class="flex flex-col">
	{#each ACCESS_ENTRY_KIND_ORDER as kind, kindIndex (kind)}
		{@const entries = scope.entries.filter((entry) => entry.kind === kind)}
		{@const KindIcon = kindIcons[kind]}
		{#if entries.length}
			<section class={kindIndex === 0 ? undefined : dense ? 'pt-1' : 'pt-2'}>
				<div class="flex items-center gap-1 text-2xs font-normal text-hint">
					<KindIcon size={12} class="shrink-0" />
					<span>{sectionLabels[kind]}</span>
				</div>
				<div class="flex flex-col pl-5">
					{#each entries as item (item.id)}
						<a
							href={item.path}
							onclick={(event) => event.preventDefault()}
							title={item.name}
							class="group inline-flex min-w-0 items-center gap-1 {dense
								? 'py-0.5'
								: 'py-1'} text-2xs text-secondary hover:text-emphasis focus-visible:text-emphasis"
						>
							<span class="truncate group-hover:underline group-focus-visible:underline"
								>{item.name}</span
							>
							<ExternalLink
								size={10}
								class="shrink-0 text-hint opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100"
							/>
						</a>
					{/each}
				</div>
			</section>
		{/if}
	{/each}
</div>
