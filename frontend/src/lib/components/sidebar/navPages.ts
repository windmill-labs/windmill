import {
	Boxes,
	Calendar,
	DollarSign,
	Eye,
	FolderOpen,
	Home,
	HardHat,
	Play,
	Pyramid,
	Unplug,
	Users
} from 'lucide-svelte'
import type { OperatorPageKey } from './operatorRoutes'

/**
 * The names the page header's breadcrumb gives the workspace's own pages, and the rows the
 * sidebar's main group is built from.
 *
 * Every entry names a page for the breadcrumb, and its `icon` goes there too. Only the entries
 * `SidebarContent`'s `MAIN_MENU_EXTRAS` also lists become sidebar links from here — Schedules,
 * HTTP routes, Workers and Audit logs are grouped differently there and carry their own row, so
 * for those four the `operatorKey` below goes unread. Keep the labels in step with those rows by
 * hand.
 */
export type NavPage = {
	label: string
	/** Path under `base`, matched against the current route by longest prefix. */
	path: string
	icon: any
	/** The `operator_settings` key that admits an operator, when the page has one. */
	operatorKey?: OperatorPageKey
}

export const NAV_PAGES: NavPage[] = [
	{ label: 'Home', path: '/', icon: Home },
	{ label: 'Runs', path: '/runs', icon: Play, operatorKey: 'runs' },
	{ label: 'Variables', path: '/variables', icon: DollarSign, operatorKey: 'variables' },
	{ label: 'Resources', path: '/resources', icon: Boxes, operatorKey: 'resources' },
	{ label: 'Assets', path: '/assets', icon: Pyramid, operatorKey: 'assets' },
	{ label: 'Folders', path: '/folders', icon: FolderOpen, operatorKey: 'folders' },
	{ label: 'Groups', path: '/groups', icon: Users, operatorKey: 'groups' },
	{ label: 'Schedules', path: '/schedules', icon: Calendar, operatorKey: 'schedules' },
	// "Triggers" is the group the sidebar files this under, not the page: the row there reads HTTP.
	{ label: 'HTTP routes', path: '/routes', icon: Unplug, operatorKey: 'triggers' },
	{ label: 'Workers', path: '/workers', icon: HardHat, operatorKey: 'workers' },
	{ label: 'Audit logs', path: '/audit_logs', icon: Eye, operatorKey: 'audit_logs' }
]

/**
 * The page a route belongs to. Longest match wins, so `/runs/f/demo/x` is still Runs, and a route
 * the table does not name falls back to its first segment (`/service_logs` → "Service logs").
 */
export function navPageFor(pathname: string, base: string): NavPage | undefined {
	const path = pathname.startsWith(base) ? pathname.slice(base.length) || '/' : pathname
	const match = NAV_PAGES.filter(
		(p) => p.path === path || (p.path !== '/' && path.startsWith(p.path + '/'))
	).sort((a, b) => b.path.length - a.path.length)[0]
	if (match) return match
	const first = path.split('/').filter(Boolean)[0]
	if (!first) return undefined
	const label = first.replace(/_/g, ' ')
	return {
		label: label.charAt(0).toUpperCase() + label.slice(1),
		path: `/${first}`,
		icon: undefined
	}
}
