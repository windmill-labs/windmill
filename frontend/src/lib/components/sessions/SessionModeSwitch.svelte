<script lang="ts">
	import { ChevronLeft, ChevronRight, MessagesSquare } from 'lucide-svelte'
	import { page } from '$app/state'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import SessionPicker from './SessionPicker.svelte'
	import { swapSidebarList } from './sessionSwitch.svelte'
	import type { SidebarList } from './sidebarList.svelte'

	let { list, isCollapsed = false }: { list: SidebarList; isCollapsed?: boolean } = $props()

	// Hovering the row opens the session menu; a click shows the session list instead. Taken
	// in the capture phase, ahead of the menu's own trigger, which would pin the menu open.
	// The menu opens and closes on hover by calling click() on that trigger: those untrusted
	// clicks are its own and pass through.
	function showSessions(e: MouseEvent) {
		if (!e.isTrusted) return
		e.preventDefault()
		e.stopPropagation()
		swapSidebarList('sessions', page.url.pathname === '/')
	}
</script>

{#if list === 'workspace'}
	<div onclickcapture={showSessions} role="none">
		<SessionPicker {isCollapsed}>
			{#snippet hoverTrigger({ trigger, unread })}
				<div class="relative">
					<MenuButton
						{trigger}
						{isCollapsed}
						icon={MessagesSquare}
						label="AI Sessions"
						class="!text-xs"
						trailingIcon={ChevronRight}
						aiId="sidebar-menu-ai-sessions"
						aiDescription="Button to show the AI sessions list in the sidebar"
					/>
					{#if unread > 0}
						<span
							class="absolute top-1 right-1 pointer-events-none inline-block w-2 h-2 rounded-full bg-surface-accent-primary"
							aria-label="{unread} unread message{unread === 1 ? '' : 's'} across all sessions"
						></span>
					{/if}
				</div>
			{/snippet}
		</SessionPicker>
	</div>
{:else}
	<MenuButton
		on:click={() => swapSidebarList('workspace', page.url.pathname === '/')}
		{isCollapsed}
		icon={ChevronLeft}
		label="Workspace"
		class="!text-xs"
		aiId="sidebar-menu-workspace"
		aiDescription="Button to show the workspace navigation in the sidebar"
	/>
{/if}
