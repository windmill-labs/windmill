import { FileCode, FileText, FolderIcon, Key, Users } from 'lucide-svelte'
import type { FilterSchemaRec } from '../FilterSearchbar.svelte'

export function buildVariablesFilterSchema({
	paths,
	owners,
	showUserFoldersFilter,
	userFoldersLabel
}: {
	paths: string[]
	owners: string[]
	showUserFoldersFilter?: boolean
	userFoldersLabel?: string
}) {
	return {
		_default_: {
			type: 'string' as const,
			hidden: true
		},
		owner: {
			type: 'oneof' as const,
			options: owners.map((s) => ({ label: s, value: s })),
			allowNegative: false,
			allowMultiple: false,
			label: 'Owner',
			icon: FolderIcon,
			description: 'Filter by owner (folder or user path)'
		},
		path: {
			type: 'oneof' as const,
			options: paths.map((s) => ({ label: s, value: s })),
			allowNegative: true,
			allowMultiple: true,
			label: 'Path',
			icon: FileCode,
			description: 'Filter by exact variable path'
		},
		path_start: {
			type: 'string' as const,
			label: 'Path prefix',
			icon: FolderIcon,
			description: 'Filter by path prefix (e.g., "f/folder/")'
		},
		description: {
			type: 'string' as const,
			label: 'Description',
			icon: FileText,
			description: 'Search in variable description'
		},
		value: {
			type: 'string' as const,
			label: 'Value',
			icon: Key,
			description: 'Search in non-secret variable values'
		},
		...(showUserFoldersFilter
			? {
					user_folders_only: {
						type: 'boolean' as const,
						label: userFoldersLabel || 'User folders only',
						icon: Users,
						description: 'Show only variables in user folders'
					}
				}
			: {})
	} satisfies FilterSchemaRec
}
