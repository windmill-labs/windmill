<script lang="ts">
	import { resource } from 'runed'
	import { twMerge } from 'tailwind-merge'
	import { Bot, MessagesSquare } from 'lucide-svelte'
	import SidebarSection from './SidebarSection.svelte'
	import { page } from '$app/state'
	import { afterNavigate } from '$app/navigation'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { FlowConversationsService, type FlowConversation } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { agentOfChatPath } from '$lib/components/flows/conversations/agentEditorChat'
	import { sidebarClasses } from './MenuButton.svelte'
	import SidebarShowOlder, { SIDEBAR_RECENT_MS } from './SidebarShowOlder.svelte'

	// The reader's own flow and agent chats, the ones a flow's or an agent's own page shows: the
	// editors' test chats stay with the editors.
	let { isCollapsed = false }: { isCollapsed?: boolean } = $props()

	const sectionCollapsed = useLocalStorageValue(
		'windmill_ai_chats_section_collapsed',
		false,
		'boolean'
	)

	const showOlder = useLocalStorageValue('windmill_ai_chats_show_older', false, 'boolean')

	// Tagged with its workspace: responses are not ordered, and a late one from the workspace
	// just left must not show here.
	const listing = resource(
		() => $workspaceStore,
		async (ws) => {
			if (!ws) return undefined
			const rows = await FlowConversationsService.listFlowConversations({
				workspace: ws,
				perPage: 30,
				kind: 'deployed',
				mine: true
			})
			// Listed newest `updated_at` first, which every new message bumps, so the chats active
			// in the window are a prefix of the page.
			const since = Date.now() - SIDEBAR_RECENT_MS
			const recent = rows.findIndex((c) => new Date(c.updated_at).getTime() < since)
			return { ws, rows, recent: recent === -1 ? rows.length : recent }
		}
	)
	// A chat is started on its flow's or agent's page, so it lists once the reader leaves one.
	afterNavigate(({ from, to }) => {
		const left = from?.url.pathname
		if (!left || left === to?.url.pathname) return
		if (left.startsWith('/flows/get/') || left.startsWith('/agents/get/')) listing.refetch()
	})
	const current = $derived(
		listing.current && listing.current.ws === $workspaceStore ? listing.current : undefined
	)
	const allRows = $derived(current?.rows ?? [])
	const openId = $derived(page.url.searchParams.get('conversation'))
	// The open chat always lists, however old.
	const rows = $derived(
		showOlder.val || !current
			? allRows
			: allRows.filter((c, i) => i < current.recent || c.id === openId)
	)
	const olderCount = $derived(
		allRows.filter((c, i) => i >= (current?.recent ?? 0) && c.id !== openId).length
	)

	function target(c: FlowConversation) {
		const agent = agentOfChatPath(c.flow_path)
		return {
			agent: agent !== undefined,
			path: agent ?? c.flow_path,
			href: `/${agent !== undefined ? 'agents' : 'flows'}/get/${agent ?? c.flow_path}?conversation=${c.id}`
		}
	}
</script>

{#if allRows.length > 0}
	<SidebarSection
		title="AI chats"
		icon={MessagesSquare}
		folded={sectionCollapsed.val}
		onToggle={() => (sectionCollapsed.val = !sectionCollapsed.val)}
		{isCollapsed}
	>
		{#snippet children(inPopover)}
			{@const rail = isCollapsed && !inPopover}
			<div
				class="flex flex-col gap-0.5 max-h-[40vh] overflow-y-auto {rail ? 'scrollbar-hidden' : ''}"
			>
				{#each rows as c (c.id)}
					{@const t = target(c)}
					{@const selected = openId === c.id}
					<a
						href={t.href}
						title="{t.agent ? 'Agent' : 'Flow'} {t.path}"
						class={twMerge(
							// shrink-0: in the capped list a row would otherwise shrink to its content,
							// which is shorter in the rail, where it has no label.
							'flex flex-row items-center gap-2 px-2 h-6 shrink-0 rounded text-xs font-normal min-w-0',
							selected
								? twMerge(sidebarClasses.selectedBg, sidebarClasses.selectedText)
								: 'text-secondary hover:bg-surface-hover'
						)}
					>
						{#if t.agent}
							<Bot size={14} class="shrink-0 text-violet-500" />
						{:else}
							<BarsStaggered size={14} class="shrink-0 text-teal-500" />
						{/if}
						<span class={rail ? 'sr-only' : 'truncate'}>{c.title || 'Untitled chat'}</span>
					</a>
				{/each}
				{#if olderCount > 0}
					<SidebarShowOlder
						count={olderCount}
						expanded={showOlder.val}
						onToggle={() => (showOlder.val = !showOlder.val)}
						{rail}
					/>
				{/if}
			</div>
		{/snippet}
	</SidebarSection>
{/if}
