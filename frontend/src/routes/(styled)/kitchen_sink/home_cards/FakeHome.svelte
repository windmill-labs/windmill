<script lang="ts">
	import { VARIANTS, type Config, type Entry, type Variant } from './model'
	import type { TreeItem, Workspace } from './fixtures'
	import { keyOf } from './format'
	import FakeSidebar from './FakeSidebar.svelte'
	import FakeHeader from './FakeHeader.svelte'
	import FakeComposer from './FakeComposer.svelte'
	import FakeItemsCard from './FakeItemsCard.svelte'
	import QuickAccess from './QuickAccess.svelte'

	interface Props {
		config: Config
		/** Remounted with a new workspace, which resets what was starred and forgotten here. */
		workspace: Workspace
		now: number
	}

	let { config, workspace, now }: Props = $props()

	// What was starred, unstarred or removed from recent in the playground, by kind/path.
	let starOverride = $state<Record<string, boolean>>({})
	let forgotten = $state<Record<string, true>>({})

	// The section's order as HomeQuickAccess builds it: favorites, then the recent edits. An
	// unstarred favorite that was never edited here leaves the section, as it does on Home.
	let entries: Entry[] = $derived.by(() => {
		const all = workspace.entries
			.filter((e) => !forgotten[keyOf(e)])
			.map((e) => ({ ...e, favorite: starOverride[keyOf(e)] ?? e.favorite }))
		for (const owner of workspace.owners) {
			for (const item of owner.items) {
				const key = `${item.kind}/${item.path}`
				if (starOverride[key] && !all.some((e) => keyOf(e) === key)) {
					all.push({ kind: item.kind, path: item.path, summary: item.summary, favorite: true })
				}
			}
		}
		return [
			...all.filter((e) => e.favorite),
			...all.filter((e) => !e.favorite && e.at).sort((a, b) => (b.at ?? 0) - (a.at ?? 0))
		]
	})
	let starred = $derived(new Set(entries.filter((e) => e.favorite).map((e) => e.path)))

	function toggleStar(e: { kind: Entry['kind']; path: string }) {
		const key = `${e.kind}/${e.path}`
		starOverride[key] = !starred.has(e.path)
	}
	function forget(e: Entry) {
		forgotten[keyOf(e)] = true
	}

	let airy = $derived(config.spacing === 'airy')
	let gap = $derived(airy ? 'pb-10' : 'pb-4')

	function section(variant: Variant) {
		return {
			variant,
			entries,
			config,
			now,
			onToggleStar: toggleStar,
			onForget: forget
		}
	}
</script>

<div class="flex h-screen bg-surface text-primary">
	<FakeSidebar dark={config.dark} />
	<div class="flex min-w-0 flex-1 flex-col">
		<FakeHeader name={workspace.name} drafts={workspace.drafts} />
		<div class="flex-1 overflow-y-auto">
			<div
				class="mx-auto w-full px-4 pb-6 sm:px-8 {config.column === 'narrow'
					? 'max-w-[960px]'
					: 'max-w-[1120px]'}"
			>
				{#if config.composer}
					<div class={airy ? 'pb-8' : 'pb-6'}><FakeComposer /></div>
				{:else}
					<div class="pt-8"></div>
				{/if}

				{#if config.view === 'sections'}
					<!-- Every variant's section under the same data and settings, to compare their
					     weight in one scroll. -->
					{#each VARIANTS as v (v.value)}
						<div class="mb-10">
							<div class="mb-3 border-b border-dashed border-border-light pb-1 text-2xs text-hint">
								<span class="font-semibold text-secondary">{v.label}</span> · {v.description}
							</div>
							{#if entries.length === 0}
								<div class="text-2xs text-hint">Nothing to show: the section is hidden.</div>
							{:else if v.value === 'inside'}
								<FakeItemsCard
									owners={workspace.owners}
									{config}
									{starred}
									onToggleStar={(i: TreeItem) => toggleStar(i)}
									preview
								>
									{#snippet band()}
										<QuickAccess {...section('inside')} />
									{/snippet}
								</FakeItemsCard>
							{:else}
								<QuickAccess {...section(v.value)} />
							{/if}
						</div>
					{/each}
				{:else}
					{#if entries.length > 0 && config.variant !== 'inside'}
						<div class={gap}>
							<QuickAccess {...section(config.variant)} />
						</div>
					{/if}
					{#if entries.length > 0 && config.variant === 'inside'}
						<FakeItemsCard
							owners={workspace.owners}
							{config}
							{starred}
							onToggleStar={(i: TreeItem) => toggleStar(i)}
						>
							{#snippet band()}
								<QuickAccess {...section('inside')} />
							{/snippet}
						</FakeItemsCard>
					{:else}
						<FakeItemsCard
							owners={workspace.owners}
							{config}
							{starred}
							onToggleStar={(i: TreeItem) => toggleStar(i)}
						/>
					{/if}
				{/if}
			</div>
		</div>
	</div>
</div>
