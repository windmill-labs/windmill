<script lang="ts">
	import { Building, MessagesSquare } from 'lucide-svelte'
	import { page } from '$app/state'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { resumeAiSessions, returnToWorkspace, swapHomeList } from './sessionSwitch.svelte'
	import type { SidebarList } from './sidebarList.svelte'

	let {
		list,
		isCollapsed = false,
		// Fired right before the switch navigates. Lets a host (the mobile menu drawer)
		// stay open across it, to show the list it switched to.
		onToggle
	}: { list: SidebarList; isCollapsed?: boolean; onToggle?: () => void } = $props()

	// The group's highlighted side. Melt moves it on click, before the navigation that
	// changes `list`, so it is pushed back to `list` once the click has played out: a
	// navigation that did not happen (an editor refusing to be left) must not leave the
	// other side lit.
	let selected: string | string[] | null | undefined = $derived(list)

	async function onSelected(next: SidebarList) {
		if (next === list) return
		if (page.url.pathname === '/') {
			swapHomeList(next, page.url.pathname + page.url.search)
		} else {
			onToggle?.()
			await (next === 'sessions' ? resumeAiSessions() : returnToWorkspace())
		}
		selected = list
	}
</script>

<!-- Each ToggleButton renders inside a Tooltip wrapper, which is the actual flex
     child of the group's track — so the buttons fill the rail width only if those
     wrappers grow. `[&>*]:flex-1` makes every direct child split the track evenly. -->
<ToggleButtonGroup
	bind:selected
	{onSelected}
	tabListClass={isCollapsed ? 'flex-col w-full [&>*]:w-full' : 'w-full [&>*]:flex-1'}
>
	{#snippet children({ item })}
		<ToggleButton
			{item}
			value="workspace"
			icon={isCollapsed ? Building : undefined}
			label="Workspace"
			iconOnly={isCollapsed}
			tooltip={isCollapsed ? 'Workspace' : undefined}
			size="sm"
			class="w-full justify-center whitespace-nowrap"
		/>
		<ToggleButton
			{item}
			value="sessions"
			icon={isCollapsed ? MessagesSquare : undefined}
			label="AI Sessions"
			iconOnly={isCollapsed}
			tooltip={isCollapsed ? 'AI Sessions (beta)' : undefined}
			size="sm"
			class="w-full justify-center whitespace-nowrap"
		/>
	{/snippet}
</ToggleButtonGroup>
