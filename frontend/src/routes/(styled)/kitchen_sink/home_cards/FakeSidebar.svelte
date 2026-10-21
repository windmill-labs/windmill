<script lang="ts">
	import {
		Boxes,
		Calendar,
		ChevronDown,
		DollarSign,
		FolderOpen,
		HardHat,
		Home,
		List,
		Play,
		Plus,
		Pyramid,
		Search,
		Settings,
		Star,
		Unplug,
		Users
	} from 'lucide-svelte'
	import { SIDEBAR_BG, SIDEBAR_BG_DARK } from '$lib/components/sidebar/sidebarChrome'

	interface Props {
		dark: boolean
	}

	let { dark }: Props = $props()

	const TOP = [
		{ icon: Home, label: 'Home', active: true },
		{ icon: Play, label: 'Runs' },
		{ icon: Star, label: 'Favorites', chevron: true },
		{ icon: Search, label: 'Search', kbd: '⌘k' }
	]
	const MIDDLE = [
		{ icon: DollarSign, label: 'Variables' },
		{ icon: Boxes, label: 'Resources' },
		{ icon: Pyramid, label: 'Assets' },
		{ icon: FolderOpen, label: 'Folders' },
		{ icon: Users, label: 'Groups' }
	]
	const TRIGGERS = [
		{ icon: Calendar, label: 'Schedules' },
		{ icon: Unplug, label: 'WebSockets' }
	]
	const BOTTOM = [
		{ icon: HardHat, label: 'Workers' },
		{ icon: List, label: 'Logs', chevron: true },
		{ icon: Settings, label: 'Settings', chevron: true }
	]
</script>

{#snippet navItem(item: {
	icon: typeof Home
	label: string
	active?: boolean
	chevron?: boolean
	kbd?: string
})}
	<div
		class="flex h-8 items-center gap-2 rounded-md px-2 text-xs {item.active
			? 'bg-surface-hover font-semibold text-emphasis'
			: 'text-primary'}"
	>
		<item.icon size={16} class="shrink-0" />
		<span>{item.label}</span>
		{#if item.kbd}<span class="text-2xs text-hint">{item.kbd}</span>{/if}
		{#if item.chevron}<ChevronDown size={14} class="ml-auto text-secondary" />{/if}
	</div>
{/snippet}

<!-- The app's sidebar as a picture, for the page's overall weight; nothing in it works. -->
<aside
	class="hidden w-52 shrink-0 flex-col shadow-[inset_-1px_0_0_0_rgb(var(--color-border-light))] md:flex"
	style:background-color={dark ? SIDEBAR_BG_DARK : SIDEBAR_BG}
	aria-hidden="true"
>
	<div class="p-2">
		<div class="flex rounded-md bg-surface-secondary p-0.5 text-xs">
			<span class="flex-1 rounded bg-surface-tertiary py-1 text-center text-emphasis">
				Workspace
			</span>
			<span class="flex-1 py-1 text-center text-secondary">AI Sessions</span>
		</div>
	</div>
	<div class="flex flex-col gap-1 px-2">
		{#each TOP as item (item.label)}{@render navItem(item)}{/each}
	</div>
	<div class="mt-3 flex flex-col gap-1 px-2">
		{#each MIDDLE as item (item.label)}{@render navItem(item)}{/each}
	</div>
	<div class="mt-3 px-2 text-3xs font-semibold uppercase tracking-wide text-secondary">
		Triggers
	</div>
	<div class="mt-1 flex flex-col gap-1 px-2">
		{#each TRIGGERS as item (item.label)}{@render navItem(item)}{/each}
		<div class="flex h-8 items-center px-2 text-secondary"><Plus size={16} /></div>
	</div>
	<div class="mt-auto flex flex-col gap-1 px-2 pb-2">
		{#each BOTTOM as item (item.label)}{@render navItem(item)}{/each}
		<div class="mt-2 flex h-8 items-center px-2 text-xs font-semibold text-emphasis">Windmill</div>
	</div>
</aside>
