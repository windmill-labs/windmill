<script lang="ts">
	import { Pen } from 'lucide-svelte'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import {
		devWorkspaceEditUrl,
		editInForkLabel,
		forkWorkspaceUrl,
		onEditInForkClick,
		type ItemType
	} from '$lib/utils/editInFork'
	import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
	import Button from '../button/Button.svelte'

	interface Props {
		itemType: ItemType
		path: string
	}

	let { itemType, path }: Props = $props()

	let dev = $derived(findCanonicalDevWorkspace($workspaceStore, $userWorkspaces))
	let label = $derived(editInForkLabel($workspaceStore, $userWorkspaces))
	// Built from the reactive `dev` rather than `buildForkEditUrl`, whose store reads are untracked:
	// the href would otherwise stay frozen at mount while the label kept updating, so a row that
	// outlives a workspace change would offer to fork a workspace that already has a dev.
	let href = $derived(
		dev ? devWorkspaceEditUrl(itemType, path, dev.id) : forkWorkspaceUrl(itemType, path)
	)
</script>

<!-- title on the wrapper, not on <Button>: Button renders its `title` prop only in the
     <button> branch, so the <a> branch taken here (href is set) would silently drop it.
     On the wrapper it also covers the icon and padding, not just the label text. -->
<div title={label}>
	<Button
		variant="subtle"
		wrapperClasses="max-w-56"
		unifiedSize="md"
		startIcon={{ icon: Pen }}
		{href}
		onClick={(e) => onEditInForkClick(e, itemType, path, { hasHref: true })}
	>
		{#if dev}
			<!-- Split so only the workspace name ellipsizes — "Edit in" always stays whole. -->
			<span class="inline-flex items-center gap-1 min-w-0">
				<span class="shrink-0">Edit in</span>
				<span class="truncate">{dev.name}</span>
			</span>
		{:else}
			<span class="truncate">{label}</span>
		{/if}
	</Button>
</div>
