<!--
@component
The Data menu's configured data tables, ducklakes and object storages, each opening its drawer
(Manage or Explore) as the Assets page's cards do. Mounted with the open menu, so the lists are
fetched on open.
-->
<script lang="ts">
	import { resource } from 'runed'
	import { Database, File } from 'lucide-svelte'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import MenuItem from '$lib/components/meltComponents/MenuItem.svelte'
	import DucklakeIcon from '$lib/components/icons/DucklakeIcon.svelte'
	import { SettingService, WorkspaceService } from '$lib/gen'
	import type { Asset } from '$lib/components/assets/lib'
	import { exploreAsset } from '$lib/components/assets/exploreAsset'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { useRoleCapabilities } from '$lib/operatorWriteRights'

	let { item, itemClass }: { item: MenubarMenuElements['item']; itemClass: string } = $props()

	const operatingWorkspace = useOperatingWorkspace()
	const roleCaps = useRoleCapabilities()

	type Store = { asset: Asset; label: string; action: 'Manage' | 'Explore' }
	type StoreGroup = { title: string; stores: Store[] }

	// A list that fails to load is left out rather than failing the menu.
	const groups = resource(
		() => ($roleCaps.code ? $operatingWorkspace : undefined),
		async (workspace): Promise<StoreGroup[]> => {
			if (!workspace) return []
			const [tables, ducklakes, storages] = await Promise.all([
				WorkspaceService.listDataTables({ workspace }).catch(() => []),
				WorkspaceService.listDucklakes({ workspace }).catch(() => []),
				SettingService.getSecondaryStorageNames({ workspace, includeDefault: true }).catch(() => [])
			])
			const all: StoreGroup[] = [
				{
					title: 'Data tables',
					stores: tables.map(({ name }): Store => ({
						asset: { kind: 'datatable', path: name },
						label: name === 'main' ? 'Main data table' : name,
						action: 'Manage'
					}))
				},
				{
					title: 'Ducklakes',
					stores: ducklakes.map((name): Store => ({
						asset: { kind: 'ducklake', path: name },
						label: name === 'main' ? 'Main ducklake' : name,
						action: 'Manage'
					}))
				},
				{
					title: 'Object storage',
					stores: storages.map((name): Store => ({
						asset: { kind: 's3object', path: name === '_default_' ? '/' : `${name}/` },
						label: name === '_default_' ? 'Primary storage' : name,
						action: 'Explore'
					}))
				}
			]
			return all.filter((g) => g.stores.length > 0)
		}
	)
</script>

<!-- Headed as DropdownV2's `sectionTitle` rows are. -->
{#each groups.current ?? [] as group (group.title)}
	<div class="my-1 border-t border-border-light"></div>
	<p class="px-4 pt-1.5 pb-1 text-2xs uppercase tracking-wide text-secondary">{group.title}</p>
	{#each group.stores as store (`${store.asset.kind}:${store.asset.path}`)}
		<MenuItem
			class={itemClass}
			{item}
			onClick={() => exploreAsset(store.asset, { workspace: $operatingWorkspace })}
		>
			<div class="flex flex-row items-center gap-2">
				{#if store.asset.kind === 'ducklake'}
					<DucklakeIcon size={16} />
				{:else if store.asset.kind === 'datatable'}
					<Database size={16} />
				{:else}
					<File size={16} />
				{/if}
				<span class="truncate">{store.label}</span>
				<span class="ml-auto pl-4 text-hint">{store.action}</span>
			</div>
		</MenuItem>
	{/each}
{/each}
