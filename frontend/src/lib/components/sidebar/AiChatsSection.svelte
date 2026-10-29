<script lang="ts">
	import { resource } from 'runed'
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import { Bot } from 'lucide-svelte'
	import SidebarSectionHeader from './SidebarSectionHeader.svelte'
	import { page } from '$app/state'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { FlowConversationsService, type FlowConversation } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { agentOfChatPath } from '$lib/components/flows/conversations/agentEditorChat'
	import { sidebarClasses } from './MenuButton.svelte'

	// The workspace's flow and agent chats, the ones a flow's or an agent's own page shows: the
	// editors' test chats stay with the editors.
	let { isCollapsed = false }: { isCollapsed?: boolean } = $props()

	const sectionCollapsed = useLocalStorageValue(
		'windmill_ai_chats_section_collapsed',
		false,
		'boolean'
	)

	// Read again on each navigation, so a chat started on a page lists once the reader moves on.
	// Tagged with its workspace: responses are not ordered, and a late one from the workspace
	// just left must not show here.
	const listing = resource(
		() => [$workspaceStore, page.url.pathname] as const,
		async ([ws]) => {
			if (!ws) return undefined
			const rows = await FlowConversationsService.listFlowConversations({
				workspace: ws,
				perPage: 30,
				kind: 'deployed'
			})
			return { ws, rows }
		}
	)
	const rows = $derived(
		listing.current && listing.current.ws === $workspaceStore ? listing.current.rows : []
	)

	function target(c: FlowConversation) {
		const agent = agentOfChatPath(c.flow_path)
		return {
			agent: agent !== undefined,
			path: agent ?? c.flow_path,
			href: `/${agent !== undefined ? 'agents' : 'flows'}/get/${agent ?? c.flow_path}?conversation=${c.id}`
		}
	}

	const openId = $derived(page.url.searchParams.get('conversation'))
</script>

{#if !isCollapsed && rows.length > 0}
	<div class="flex flex-col gap-1">
		<SidebarSectionHeader
			title="AI chats"
			collapsed={sectionCollapsed.val}
			onToggle={() => (sectionCollapsed.val = !sectionCollapsed.val)}
		/>
		{#if !sectionCollapsed.val}
			<div
				transition:slide={{ duration: 180 }}
				class="flex flex-col gap-0.5 max-h-[40vh] overflow-y-auto"
			>
				{#each rows as c (c.id)}
					{@const t = target(c)}
					{@const selected = openId === c.id}
					<a
						href={t.href}
						title="{t.agent ? 'Agent' : 'Flow'} {t.path}"
						class={twMerge(
							'flex flex-row items-center gap-2 px-2 py-1 rounded text-xs font-normal min-w-0',
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
						<span class="truncate">{c.title || 'Untitled chat'}</span>
					</a>
				{/each}
			</div>
		{/if}
	</div>
{/if}
