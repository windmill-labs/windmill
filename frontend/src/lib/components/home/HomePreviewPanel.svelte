<script module lang="ts">
	import { preloadComponent } from '$lib/utils/preloadComponent.svelte'

	// A tab renders a whole item through most of AIChat's code, monaco among it, which Home's
	// static imports may not reach.
	const previewTabHostComponent = preloadComponent(
		() => import('$lib/components/sessions/PreviewTabHost.svelte')
	)
</script>

<script lang="ts">
	import { MonitorPlay, PanelRightClose, PanelRightOpen } from 'lucide-svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import { goto } from '$lib/navigation'
	import { Button } from '$lib/components/common'
	import DraggableTabs, { type TabItem } from '$lib/components/common/tabs/DraggableTabs.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import { homeComposerContext } from '$lib/components/sessions/homeComposerContext.svelte'
	import { onPreviewFrameMessage } from '$lib/components/sessions/previewFrameMessages'
	import { previewLocationLabel } from '$lib/components/sessions/previewRouter'
	import { withWorkspaceParam } from '$lib/components/sessions/sessionMode.svelte'
	import {
		sessionState,
		type SessionPreviewTab
	} from '$lib/components/sessions/sessionState.svelte'
	import type { SessionPreviewTabs } from '$lib/components/sessions/sessionPreviewTabs.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { workspaceStore } from '$lib/stores'

	// Home's side of the session page's preview panel: the tabs of the draft in Home's composer,
	// so a send hands them to the session page with the rest of the draft. Kept to tabs, close
	// and collapse; the pickers and full screen are the session page's.

	const runtime = $derived(homeComposerContext.runtime)
	const owner: SessionPreviewTabs | undefined = $derived(runtime?.previewTabs)
	const session = $derived(sessionState.sessions.find((s) => s.id === runtime?.sessionId))
	const shown = $derived(!!owner && !owner.collapsed)
	const isDarkMode = useIsDarkMode()

	// The active tab's content is mounted, and so are the last few left through the strip: each
	// is a whole Windmill page or editor.
	const MAX_MOUNTED_TABS = 5
	const shownTabIds = new SvelteSet<string>()
	function noteShown(id: string) {
		shownTabIds.delete(id)
		shownTabIds.add(id)
		while (shownTabIds.size > MAX_MOUNTED_TABS) {
			const oldest = shownTabIds.values().next().value
			if (oldest === undefined || oldest === id) break
			shownTabIds.delete(oldest)
		}
	}

	function label(tab: SessionPreviewTab): string {
		return tab.friendlyLabel ?? previewLocationLabel(tab.loc || tab.url)
	}
	const tabItems = $derived<TabItem[]>(
		(owner?.tabs ?? []).map((t) => ({ id: t.id, label: label(t), title: label(t) }))
	)

	function selectTab(id: string) {
		if (owner?.activeId) noteShown(owner.activeId)
		owner?.select(id)
	}
	function closeTab(id: string) {
		owner?.close(id)
		shownTabIds.delete(id)
	}

	function navigateEditorTo(item: WorkspaceItem) {
		// A drag-and-drop app has no preview wrapper: it opens in its own editor.
		if (item.kind === 'app' && !item.raw_app) {
			void goto(withWorkspaceParam(`/apps/edit/${item.path}`, $workspaceStore ?? undefined))
			return
		}
		owner?.navigate({ type: 'item', item })
	}

	function observeLocation(tab: SessionPreviewTab, frame: HTMLIFrameElement) {
		try {
			const win = frame.contentWindow
			if (!win) return
			const observe = () => {
				try {
					owner?.observeLocation(
						tab.id,
						win.location.pathname + win.location.search + win.location.hash
					)
				} catch {
					// Mid-navigation the location can throw; the tab keeps its last one.
				}
			}
			observe()
			win.addEventListener('hashchange', observe)
			win.addEventListener('popstate', observe)
		} catch {
			// Same as above.
		}
	}
</script>

<svelte:window onmessage={(e) => shown && onPreviewFrameMessage(e, owner)} />

{#if owner && !shown}
	<PageHeaderContent actions={openButton} />
{/if}

{#snippet openButton()}
	<Button
		variant="subtle"
		unifiedSize="sm"
		startIcon={{ icon: PanelRightOpen }}
		title="Open side panel"
		onclick={() => owner?.setCollapsed(false)}
	>
		Open side panel
	</Button>
{/snippet}

{#if owner && shown}
	<div class="w-[45%] min-w-80 shrink-0 h-full p-2 pl-0 flex flex-col min-h-0">
		<div
			class="flex flex-col flex-1 min-h-0 overflow-hidden relative bg-surface rounded-md border border-light"
		>
			<button
				type="button"
				onclick={() => owner?.setCollapsed(true)}
				title="Collapse preview"
				aria-label="Collapse preview"
				class="absolute top-1 left-1 z-30 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
			>
				<PanelRightClose size={14} />
			</button>
			<DraggableTabs
				tabs={tabItems}
				activeId={owner.activeId}
				onSelect={selectTab}
				onClose={closeTab}
				onReorder={(next) => owner?.reorder(next.map((t) => t.id))}
				size="sm"
				class="session-preview-tab-strip h-8 border-b border-light bg-surface-secondary/50 pl-9 pr-2"
			/>
			<div class="relative flex-1 min-h-0">
				{#if previewTabHostComponent.current}
					{@const TabHost = previewTabHostComponent.current}
					{#each owner.tabs as tab (tab.id)}
						{@const active = tab.id === owner.activeId}
						<TabHost
							{tab}
							{session}
							{runtime}
							{active}
							mounted={active || shownTabIds.has(tab.id)}
							label={label(tab)}
							darkMode={isDarkMode.val}
							onNavigate={navigateEditorTo}
							onSeeDetails={(item) => owner?.open({ type: 'item', item, mode: 'view' })}
							onLoad={(frame) => observeLocation(tab, frame)}
						/>
					{/each}
				{/if}
				{#if owner.tabs.length === 0}
					<div
						class="absolute inset-0 flex flex-col items-center justify-center gap-2 text-center px-6 bg-surface"
					>
						<MonitorPlay size={28} class="text-tertiary" />
						<span class="text-sm font-medium text-secondary">No preview open</span>
						<span class="text-xs text-tertiary max-w-xs">
							Click a script, flow or app on Home to preview it here.
						</span>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
