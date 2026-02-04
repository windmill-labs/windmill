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
		const publicDatatableRegex = /datatable:\/\/.*\/public\.[^/.]+/
		const publicDucklakeRegex = /ducklake:\/\/.*\/main\.[^/.]+/
		if (kind === 'asset' && path.match(publicDatatableRegex)) return path.replace('/public.', '/')
		if (kind === 'asset' && path.match(publicDucklakeRegex)) return path.replace('/main.', '/')

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

		async star(path: string, favorite_kind: FavoriteKind, workspaceId?: string) {
			const href = getFavoriteHref(path, favorite_kind)
			const label = getFavoriteLabel(path, favorite_kind)
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
</script>

<script lang="ts">
	import { CodeXml, LayoutDashboard, Pyramid, Star } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { Menu, MenuItem } from '$lib/components/meltComponents'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import type { MenubarBuilders } from '@melt-ui/svelte'
	import { FavoriteService, type StarData } from '$lib/gen'
	import { get } from 'svelte/store'
	import { globalDbManagerDrawer, workspaceStore } from '$lib/stores'
	import { type DbInput } from '../dbTypes'
	import { parseDbInputFromAssetSyntax } from '$lib/utils'

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
						<MenuItem
							href={favorite.href}
							onClick={() => {
								if (favorite.kind === 'asset') {
									const dbInput = parseDbInputFromAssetSyntax(favorite.path)
									if (dbInput) globalDbManagerDrawer.val?.openDrawer(dbInput)
								}
							}}
							{item}
							class="w-full inline-flex flex-row px-4 py-2 data-[highlighted]:bg-surface-hover"
						>
							<span class="center-center">
								{#if favorite.kind == 'script'}
									<CodeXml size={16} />
								{:else if favorite.kind == 'flow'}
									<BarsStaggered size={16} />
								{:else if favorite.kind == 'app' || favorite.kind == 'raw_app'}
									<LayoutDashboard size={16} />
								{:else if favorite.kind == 'asset'}
									<Pyramid size={16} />
								{/if}
							</span>
							<span class="text-primary ml-2 grow min-w-0 text-xs truncate">
								{favorite.label}
							</span>
						</MenuItem>
					{/each}
				</div>
			{/if}
		</div>
	{/snippet}
</Menu>
