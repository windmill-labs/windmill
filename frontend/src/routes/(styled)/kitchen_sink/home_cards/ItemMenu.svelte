<script lang="ts">
	import { Copy, Eye, GitFork, List, Pen, Star, StarOff, X } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import type { Item } from '$lib/utils'
	import type { Entry } from './model'

	interface Props {
		entry: Entry
		onToggleStar: (entry: Entry) => void
		onForget: (entry: Entry) => void
	}

	let { entry, onToggleStar, onForget }: Props = $props()

	// The real card's menu, minus the navigation: the playground has nowhere to go.
	function items(): Item[] {
		return [
			{ displayName: 'Edit', icon: Pen, action: () => {} },
			{
				displayName: 'Duplicate/Fork',
				icon: GitFork,
				action: () => {},
				hide: entry.kind === 'agent'
			},
			{
				displayName: 'View runs',
				icon: List,
				action: () => {},
				hide: entry.kind === 'app' || entry.kind === 'raw_app'
			},
			{ displayName: 'Audit logs', icon: Eye, action: () => {} },
			{ displayName: 'Copy path', icon: Copy, action: () => {} },
			{
				displayName: entry.favorite ? 'Remove from favorites' : 'Add to favorites',
				icon: entry.favorite ? StarOff : Star,
				separatorTop: true,
				action: () => onToggleStar(entry)
			},
			{
				displayName: 'Remove from recent',
				icon: X,
				action: () => onForget(entry),
				hide: entry.favorite
			}
		]
	}
</script>

<DropdownV2 {items} size="xs" fixedHeight={false} />
