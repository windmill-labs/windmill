<script lang="ts">
	import { Building, MessagesSquare } from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { enterSessionMode, exitSessionMode } from './sessionSwitch.svelte'
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { base } from '$lib/base'

	// Which side of the switch is active. `nav` = workspace navigation (the classic
	// app), `session` = the sessions sidebar + chat + preview.
	let {
		mode,
		isCollapsed = false,
		// Fired right before the toggle navigates. Lets a host (e.g. the mobile menu
		// drawer) keep itself open across the mode switch.
		onToggle
	}: { mode: 'nav' | 'session'; isCollapsed?: boolean; onToggle?: () => void } = $props()

	function onSelected(next: 'nav' | 'session') {
		if (next === mode) return
		onToggle?.()
		if (next === 'session') void enterSessionMode()
		else void exitSessionMode()
	}

	// Pressing the already-active "Workspace" side goes home, so the toggle doubles
	// as the home button when there is no mode to switch to. `onToggle` is
	// deliberately not fired: this is a plain in-mode navigation, so the mobile menu
	// drawer should dismiss like it does for any other nav link.
	function onNavActivate() {
		if (mode !== 'nav') return
		// `goto` has no same-URL short-circuit, so navigating from home would push a
		// duplicate history entry and make the next Back press look broken.
		if (page.url.pathname === `${base}/`) return
		void goto('/')
	}
</script>

<!-- Each ToggleButton renders inside a Tooltip wrapper, which is the actual flex
     child of the group's track — so the buttons fill the rail width only if those
     wrappers grow. `[&>*]:flex-1` makes every direct child split the track evenly. -->
<ToggleButtonGroup
	selected={mode}
	{onSelected}
	tabListClass={isCollapsed ? 'flex-col w-full [&>*]:w-full' : 'w-full [&>*]:flex-1'}
>
	{#snippet children({ item })}
		<ToggleButton
			{item}
			value="nav"
			icon={isCollapsed ? Building : undefined}
			label="Workspace"
			iconOnly={isCollapsed}
			tooltip={isCollapsed ? 'Workspace' : undefined}
			size="sm"
			class="w-full justify-center"
			onActivate={onNavActivate}
		/>
		<ToggleButton
			{item}
			value="session"
			icon={isCollapsed ? MessagesSquare : undefined}
			label="AI Sessions"
			iconOnly={isCollapsed}
			tooltip={isCollapsed ? 'AI Sessions (beta)' : undefined}
			size="sm"
			class="w-full justify-center"
		/>
	{/snippet}
</ToggleButtonGroup>
