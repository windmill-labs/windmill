<script lang="ts">
	import { House, MessagesSquare } from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { enterSessionMode, exitSessionMode } from './sessionSwitch.svelte'

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
			icon={isCollapsed ? House : undefined}
			label="Workspace"
			iconOnly={isCollapsed}
			tooltip={isCollapsed ? 'Workspace' : undefined}
			size="sm"
			class="w-full justify-center"
		/>
		<ToggleButton
			{item}
			value="session"
			icon={isCollapsed ? MessagesSquare : undefined}
			label="AI Sessions"
			iconOnly={isCollapsed}
			tooltip={isCollapsed ? 'AI Sessions' : undefined}
			size="sm"
			class="w-full justify-center"
		/>
	{/snippet}
</ToggleButtonGroup>
