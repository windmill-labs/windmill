<script lang="ts" module>
	import {
		AlignLeft,
		Calendar,
		Database,
		FileCode,
		LayoutDashboard,
		Variable,
		Workflow
	} from 'lucide-svelte'

	export type ResourceKind = 'flow' | 'script' | 'app' | 'schedule' | 'resource' | 'variable'

	export type KindMeta = {
		id: ResourceKind
		label: string
		labelPlural: string
		icon: typeof Workflow
	}

	export const RESOURCE_KINDS: KindMeta[] = [
		{ id: 'flow', label: 'Flow', labelPlural: 'Flows', icon: AlignLeft },
		{ id: 'script', label: 'Script', labelPlural: 'Scripts', icon: FileCode },
		{ id: 'app', label: 'App', labelPlural: 'Apps', icon: LayoutDashboard },
		{ id: 'schedule', label: 'Schedule', labelPlural: 'Schedules', icon: Calendar },
		{ id: 'resource', label: 'Resource', labelPlural: 'Resources', icon: Database },
		{ id: 'variable', label: 'Variable', labelPlural: 'Variables', icon: Variable }
	]
</script>

<script lang="ts">
	import { Folder } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Item } from '$lib/utils'
	import Dropdown from '$lib/components/DropdownV2.svelte'

	type FolderEntry = { path: string; label?: string }
	type ItemEntry = { path: string; summary?: string }

	interface Props {
		kind: ResourceKind
		path: string
		summary?: string
		// Optional providers — design preview uses inline mocks if not given.
		folders?: FolderEntry[]
		// Items visible across all folders for the current kind. The component
		// filters them per-folder when building submenus.
		items?: ItemEntry[]
		onSelectKind?: (kind: ResourceKind) => void
		onSelectFolder?: (folderPath: string, kind: ResourceKind) => void
		onSelectItem?: (item: ItemEntry, kind: ResourceKind) => void
	}

	let { kind, path, summary, folders, items, onSelectKind, onSelectFolder, onSelectItem }: Props =
		$props()

	const breadcrumbParts = $derived((path ?? '').split('/').filter(Boolean))
	const folderPath = $derived(
		breadcrumbParts.length > 1 ? breadcrumbParts.slice(0, -1).join('/') : ''
	)
	const leaf = $derived(
		breadcrumbParts.length > 0 ? breadcrumbParts[breadcrumbParts.length - 1] : 'untitled'
	)
	const currentKind = $derived(RESOURCE_KINDS.find((k) => k.id === kind) ?? RESOURCE_KINDS[0])

	// --- design-preview fallbacks ---------------------------------------------
	const fallbackFolders: FolderEntry[] = [
		{ path: 'u/admin', label: 'admin' },
		{ path: 'u/guilhempw', label: 'guilhempw' },
		{ path: 'f/team', label: 'team' },
		{ path: 'f/data', label: 'data' },
		{ path: 'f/integrations', label: 'integrations' }
	]
	const fallbackItemsByKind: Record<ResourceKind, ItemEntry[]> = {
		flow: [
			{ path: 'u/admin/sandbox_flow', summary: 'Sandbox flow' },
			{ path: 'u/admin/payment_processor', summary: 'Payment processor' },
			{ path: 'u/guilhempw/dashboard_data', summary: 'Dashboard data sync' },
			{ path: 'f/team/onboarding', summary: 'Team onboarding' }
		],
		script: [
			{ path: 'u/admin/fetch_users', summary: 'Fetch users' },
			{ path: 'u/admin/notify_email', summary: 'Notify by email' },
			{ path: 'f/data/csv_import', summary: 'CSV import' }
		],
		app: [
			{ path: 'u/admin/admin_dashboard', summary: 'Admin dashboard' },
			{ path: 'f/team/team_directory', summary: 'Team directory' }
		],
		schedule: [
			{ path: 'u/guilhempw/fake_flow_schedule', summary: 'Fake flow schedule' },
			{ path: 'u/admin/daily_report', summary: 'Daily report' }
		],
		resource: [
			{ path: 'u/admin/postgres_main', summary: 'Postgres main' },
			{ path: 'u/admin/s3_assets', summary: 'S3 assets' }
		],
		variable: [
			{ path: 'u/admin/api_key', summary: 'API key' },
			{ path: 'u/admin/sentry_dsn', summary: 'Sentry DSN' }
		]
	}

	const effectiveFolders = $derived(folders ?? fallbackFolders)
	function itemsForKind(k: ResourceKind): ItemEntry[] {
		return items ?? fallbackItemsByKind[k] ?? []
	}
	function itemsInFolder(k: ResourceKind, folder: string): ItemEntry[] {
		return itemsForKind(k).filter((it) => {
			const parts = it.path.split('/').filter(Boolean)
			const f = parts.length > 1 ? parts.slice(0, -1).join('/') : ''
			return f === folder
		})
	}

	// --------------------------------------------------------------------------
	// Item-tree builders. Each level's segment opens a dropdown rooted at that
	// level — clicking a kind cascades through (kind > folder > item), clicking
	// a folder cascades through (folder > item), clicking the leaf jumps
	// straight to the item list.

	function buildItemEntries(k: ResourceKind, folder: string): Item[] {
		const list = itemsInFolder(k, folder)
		if (list.length === 0) {
			return [{ displayName: 'No items in this folder', disabled: true }]
		}
		return list.map(
			(it): Item => ({
				displayName: it.summary || it.path.split('/').pop() || it.path,
				action: () => onSelectItem?.(it, k)
			})
		)
	}

	function buildFolderEntries(k: ResourceKind): Item[] {
		return effectiveFolders.map(
			(f): Item => ({
				displayName: f.label || f.path,
				icon: Folder,
				submenuItems: buildItemEntries(k, f.path)
			})
		)
	}

	const kindLevelItems = $derived(
		RESOURCE_KINDS.map(
			(k): Item => ({
				displayName: k.labelPlural,
				icon: k.icon,
				submenuItems: buildFolderEntries(k.id),
				action: () => onSelectKind?.(k.id)
			})
		)
	)
	const folderLevelItems = $derived(buildFolderEntries(kind))
	const leafLevelItems = $derived(buildItemEntries(kind, folderPath))

	// --------------------------------------------------------------------------
	const segmentBtn =
		'flex flex-row items-center gap-1.5 px-1.5 py-0.5 rounded-md hover:bg-surface-hover transition-colors text-primary text-xs min-w-0'
</script>

<div class="flex flex-row items-center gap-0.5 text-xs min-w-0">
	<!-- Level 0: kind. Cascades 3 levels deep (kind > folder > item). -->
	<Dropdown
		items={kindLevelItems}
		placement="bottom-start"
		fixedHeight={false}
		class="!justify-start"
	>
		{#snippet buttonReplacement()}
			{@const CurrentKindIcon = currentKind.icon}
			<span class={segmentBtn} title="Browse {currentKind.labelPlural}">
				<CurrentKindIcon class="w-3.5 h-3.5 text-secondary shrink-0" />
				<span class="text-secondary">{currentKind.labelPlural}</span>
			</span>
		{/snippet}
	</Dropdown>

	{#if folderPath}
		<span class="text-secondary opacity-60 shrink-0 select-none">/</span>
		<!-- Level 1: folder. Cascades 2 levels deep (folder > item). -->
		<Dropdown
			items={folderLevelItems}
			placement="bottom-start"
			fixedHeight={false}
			class="!justify-start"
		>
			{#snippet buttonReplacement()}
				<span class={twMerge(segmentBtn)} title="Browse folder {folderPath}">
					<span class="text-secondary truncate font-mono">{folderPath}</span>
				</span>
			{/snippet}
		</Dropdown>
	{/if}

	<span class="text-secondary opacity-60 shrink-0 select-none">/</span>
	<!-- Level 2: name / summary. Just the item list, no submenus. -->
	<Dropdown
		items={leafLevelItems}
		placement="bottom-start"
		fixedHeight={false}
		class="!justify-start"
	>
		{#snippet buttonReplacement()}
			<span class={twMerge(segmentBtn, 'font-medium text-primary')} title={summary || leaf}>
				<span class="truncate">{summary || leaf}</span>
			</span>
		{/snippet}
	</Dropdown>
</div>
