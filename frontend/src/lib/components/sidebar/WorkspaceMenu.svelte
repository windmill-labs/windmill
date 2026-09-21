<script lang="ts">
	import { userWorkspaces, workspaceStore, workspaceColor } from '$lib/stores'
	import { NameIdTooltip } from '$lib/components/common'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import { Menu } from '$lib/components/meltComponents'
	import { Building } from 'lucide-svelte'
	import { getContrastTextColor } from '$lib/utils'
	import type { MenubarBuilders } from '@melt-ui/svelte'
	import { ambiguousWorkspaceNames, findWorkspaceRoot } from '$lib/utils/workspaceHierarchy'
	import WorkspacePickerBody from './WorkspacePickerBody.svelte'

	interface Props {
		isCollapsed?: boolean
		createMenu: MenubarBuilders['createMenu']
		// When used outside of the side bar, where links to workspace settings and such don't make as much sense.
		strictWorkspaceSelect?: boolean
	}

	let { isCollapsed = false, createMenu, strictWorkspaceSelect = false }: Props = $props()

	// The active workspace's family root — shown in the trigger so a forked active workspace still
	// surfaces its family name here (the fork itself is shown in the breadcrumb). Resolved exactly
	// like the scope picker right below, so the two never name different heads for one workspace:
	// inside a dev workspace's own subtree the head is that dev workspace, not the far root.
	const currentFamily = $derived(
		findWorkspaceRoot($workspaceStore ?? undefined, $userWorkspaces ?? [])
	)

	const ambiguousNames = $derived(ambiguousWorkspaceNames($userWorkspaces))
</script>

<Menu {createMenu} usePointerDownOutside placement="bottom-start">
	{#snippet triggr({ trigger })}
		<!-- Family header reflects the family (root) color, not the active
		     workspace's — switching into a fork must not recolor it. -->
		{@const familyColor = currentFamily?.color ?? $workspaceColor}
		{@const iconColor = getContrastTextColor(familyColor)}
		<!-- Collapsed mode already shows MenuButton's own right-side popover, so the
		     rich tooltip only takes over when expanded. -->
		<NameIdTooltip
			name={currentFamily?.name ?? $workspaceStore ?? ''}
			id={currentFamily?.id ?? $workspaceStore ?? ''}
			disablePopup={isCollapsed}
			class="block w-full"
		>
			<MenuButton
				icon={Building}
				iconProps={iconColor ? { style: `color: ${iconColor}` } : undefined}
				label={currentFamily?.name ?? $workspaceStore ?? ''}
				sublabel={!isCollapsed && currentFamily && ambiguousNames.has(currentFamily.name)
					? currentFamily.id
					: undefined}
				{isCollapsed}
				color={familyColor}
				showChevron
				emphasizeLabel
				disableTitle
				{trigger}
			/>
		</NameIdTooltip>
	{/snippet}

	{#snippet children({ item })}
		<!-- The only strict caller is a standalone page (svix webhook creation) that owns its
		     own navigation, so there a switch must leave the page where it is. -->
		<WorkspacePickerBody {item} {strictWorkspaceSelect} keepPageOnSwitch={strictWorkspaceSelect} />
	{/snippet}
</Menu>
