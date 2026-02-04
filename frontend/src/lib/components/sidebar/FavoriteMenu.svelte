<script module lang="ts">
	export type FavoriteKind = Exclude<
		NonNullable<StarData['requestBody']>['favorite_kind'],
		undefined
	>

	export function getFavoriteHref(path: string, kind: FavoriteKind) {
		return {
			script: `/scripts/get/${path}`,
			flow: `/flows/get/${path}`,
			app: `/apps/get/${path}`,
			raw_app: `/apps_raw/get/${path}`,
			asset: `#dbmanager:${path}`
		}[kind]
	}
	export function getFavoriteLabel(path: string, kind: FavoriteKind): string {
		if (kind === 'asset') {
			const asset = parseFavoriteAsset(path)
			return `${asset.schema ? `${asset.schema}.` : ''}${asset.table} (${asset.assetName} ${asset.scheme})`
		}

		return path
	}

	class FavoriteManager {
		current: {
			label: string
			path: string
			href: string
			kind: FavoriteKind
		}[] = $state([])

		async unstar(path: string, favorite_kind: FavoriteKind, workspaceId?: string) {
			this.current = this.current.filter(
				(fav) => !(fav.path === path && fav.kind === favorite_kind)
			)

			await FavoriteService.unstar({
				workspace: workspaceId ?? get(workspaceStore)!,
				requestBody: { path, favorite_kind }
			})
		}

		async star(path: string, favorite_kind: FavoriteKind, workspaceId?: string, summary?: string) {
			const href = getFavoriteHref(path, favorite_kind)
			const label = summary || getFavoriteLabel(path, favorite_kind)
			this.current = [...this.current, { href, kind: favorite_kind, label, path }]
			await FavoriteService.star({
				workspace: workspaceId ?? get(workspaceStore)!,
				requestBody: { path, favorite_kind }
			})
		}

		isStarred(path: string, favorite_kind: string) {
			return this.current.some((fav) => fav.path === path && fav.kind === favorite_kind)
		}
	}

	export const favoriteManager = new FavoriteManager()

	export function parseFavoriteAsset(path: string) {
		const [scheme, t0] = path.split('://')
		const [assetName, tableKey] = t0.split('/')
		const [t1, t2] = tableKey.split('.')
		const [table, schema] = [
			t2 ?? t1,
			t1 === 'main' || t1 === 'public' ? undefined : t2 ? t1 : undefined
		]
		return { table, schema, assetName: assetName || 'main', path, scheme }
	}
</script>

<script lang="ts">
	import { CodeXml, LayoutDashboard, Table2, Star, Trash2 } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { Menu, MenuItem } from '$lib/components/meltComponents'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import type { MenubarBuilders } from '@melt-ui/svelte'
	import { FavoriteService, type StarData } from '$lib/gen'
	import { get } from 'svelte/store'
	import { globalDbManagerDrawer, workspaceStore } from '$lib/stores'
	import { parseDbInputFromAssetSyntax } from '$lib/utils'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'

	interface Props {
		lightMode?: boolean
		isCollapsed?: boolean
		favoriteLinks?: {
			label: string
			href: string
			path: string
			kind: FavoriteKind
		}[]
		createMenu: MenubarBuilders['createMenu']
	}

	let { lightMode = false, isCollapsed = false, favoriteLinks = [], createMenu }: Props = $props()
</script>

<Menu {createMenu} usePointerDownOutside>
	{#snippet triggr({ trigger })}
		<MenuButton
			class="!text-xs"
			icon={Star}
			label={'Favorites'}
			{isCollapsed}
			{lightMode}
			{trigger}
		/>
	{/snippet}

	{#snippet children({ item })}
		<div class="overflow-hidden" role="none">
			{#if !favoriteLinks.length}
				<div class="py-1" role="none">
					<div class="text-secondary block px-4 py-2 text-xs" role="menuitem" tabindex="-1">
						Star items first
					</div>
				</div>
			{:else}
				<div class="py-1 w-full max-w-full">
					{#each favoriteLinks ?? [] as favorite}
						<div class="relative group">
							<MenuItem
								href={favorite.href}
								onClick={() => {
									if (favorite.kind === 'asset') {
										const dbInput = parseDbInputFromAssetSyntax(favorite.path)
										if (dbInput) globalDbManagerDrawer.val?.openDrawer(dbInput)
									}
								}}
								{item}
								class="w-full inline-flex flex-row px-4 py-2 pr-8 data-[highlighted]:bg-surface-hover"
							>
								<span class="center-center">
									{#if favorite.kind == 'script'}
										<CodeXml size={16} />
									{:else if favorite.kind == 'flow'}
										<BarsStaggered size={16} />
									{:else if favorite.kind == 'app' || favorite.kind == 'raw_app'}
										<LayoutDashboard size={16} />
									{:else if favorite.kind == 'asset'}
										<Table2 size={16} />
									{/if}
								</span>
								<span class="text-primary ml-2 grow min-w-0 text-xs truncate">
									{favorite.label}
								</span>
							</MenuItem>
							<div
								class="absolute right-0 top-0 h-full flex items-center pr-1 opacity-0 group-hover:opacity-100 transition-opacity"
							>
								<DropdownV2
									items={[
										{
											displayName: 'Delete',
											icon: Trash2,
											action: () => favoriteManager.unstar(favorite.path, favorite.kind)
										}
									]}
									size="xs"
									fixedHeight={false}
								/>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/snippet}
</Menu>
