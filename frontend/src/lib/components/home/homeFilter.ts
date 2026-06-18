import { Folder, Tag, Archive, Library, Users } from 'lucide-svelte'
import type { FilterSchemaRec, FilterInstanceRec } from '../FilterSearchbar.svelte'

export function buildHomeFilterSchema({
	owners,
	labels,
	showUserFoldersFilter,
	userFoldersLabel
}: {
	owners: string[]
	labels: string[]
	showUserFoldersFilter?: boolean
	userFoldersLabel?: string
}) {
	return {
		_default_: {
			type: 'string' as const,
			hidden: true
		},
		path: {
			type: 'oneof' as const,
			options: owners.map((o) => ({ label: o, value: o })),
			allowCustomValue: true,
			allowNegative: false,
			allowMultiple: false,
			label: 'Owner / folder',
			icon: Folder,
			description: 'Filter by owner or folder prefix (e.g. u/alice or f/team)'
		},
		label: {
			type: 'oneof' as const,
			options: labels.map((l) => ({ label: l, value: l })),
			allowNegative: false,
			allowMultiple: true,
			label: 'Label',
			icon: Tag,
			description: 'Filter by label (comma-separated to require multiple)'
		},
		archived: {
			type: 'boolean' as const,
			label: 'Only archived',
			icon: Archive,
			description: 'Show only archived items'
		},
		exclude_library: {
			type: 'boolean' as const,
			label: 'Exclude library scripts',
			icon: Library,
			description: 'Hide scripts without a main function'
		},
		...(showUserFoldersFilter
			? {
					user_folders_only: {
						type: 'boolean' as const,
						label: userFoldersLabel || 'User folders only',
						icon: Users,
						description: 'Show only items in user folders'
					}
				}
			: {})
	} satisfies FilterSchemaRec
}

export type HomeFilterSchema = ReturnType<typeof buildHomeFilterSchema>
export type HomeFilterValue = Partial<FilterInstanceRec<HomeFilterSchema>>

export type LatestItem = {
	type: 'script' | 'flow' | 'app' | 'raw_app'
	path: string
	summary?: string
	time?: number
	starred?: boolean
	hash?: string
	draft_only?: boolean
	raw_app?: boolean
	workspace_id?: string
}
