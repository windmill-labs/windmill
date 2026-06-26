import { base } from '$lib/base'
import {
	Home,
	Play,
	DollarSign,
	Boxes,
	Calendar,
	Database,
	FolderOpen,
	Users,
	Settings,
	ScrollText
} from 'lucide-svelte'
import type { DrillIcon } from '$lib/components/drillPicker'
import type { WorkspaceItem } from '$lib/components/workspacePicker'

/** What the preview breadcrumb picker can route to: a static workspace page
 * or a workspace item (script/flow/app). The sessions page turns either into
 * an iframe URL. */
export type PreviewTarget =
	| { type: 'page'; href: string; label: string }
	| { type: 'item'; item: WorkspaceItem }

export type PreviewPage = { label: string; path: string; icon: DrillIcon }

// Core workspace-level destinations the preview can route to. Intentionally
// curated — the main pages, not every trigger sub-page (those live behind
// EE/feature gating in SidebarContent and aren't worth duplicating here).
export const PREVIEW_PAGES: PreviewPage[] = [
	{ label: 'Home', path: '/', icon: Home },
	{ label: 'Runs', path: '/runs', icon: Play },
	{ label: 'Variables', path: '/variables', icon: DollarSign },
	{ label: 'Resources', path: '/resources', icon: Boxes },
	{ label: 'Schedules', path: '/schedules', icon: Calendar },
	{ label: 'Assets', path: '/assets', icon: Database },
	{ label: 'Folders', path: '/folders', icon: FolderOpen },
	{ label: 'Groups', path: '/groups', icon: Users },
	{ label: 'Workspace settings', path: '/workspace_settings', icon: Settings },
	{ label: 'Audit logs', path: '/audit_logs', icon: ScrollText }
]

export const pageKey = (path: string) => `page:${path}`
export const pageHref = (path: string) => `${base}${path}`

/** Strip the deployment base prefix (and any query/hash) from a preview path
 * so it can be matched against `PREVIEW_PAGES` / parsed as an item route. */
export function stripBase(path: string): string {
	let p = path.split('?')[0].split('#')[0]
	if (base && p.startsWith(base)) p = p.slice(base.length)
	return p || '/'
}

// Match a base-stripped preview pathname to a known page, for breadcrumb
// labelling + picker highlight. Exact match; '/' only matches home.
export function matchPreviewPage(path: string): PreviewPage | undefined {
	const clean = stripBase(path)
	return PREVIEW_PAGES.find((p) => p.path === clean)
}
