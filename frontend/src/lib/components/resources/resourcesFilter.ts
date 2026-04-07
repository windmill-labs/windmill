import { Boxes, FileCode, FileText, FolderIcon, Braces, Users, Tag } from 'lucide-svelte'
import type { FilterSchemaRec } from '../FilterSearchbar.svelte'

export function buildResourcesFilterSchema({
	paths,
	resourceTypes,
	owners,
	labels,
	showUserFoldersFilter,
	userFoldersLabel
}: {
	paths: string[]
	resourceTypes: string[]
	owners: string[]
	labels?: string[]
	showUserFoldersFilter?: boolean
	userFoldersLabel?: string
}) {
	return {
		_default_: {
			type: 'string' as const,
			hidden: true
		},
		resource_type: {
			type: 'oneof' as const,
			options: resourceTypes.map((s) => ({ label: s, value: s })),
			allowNegative: false,
			allowMultiple: true,
			label: 'Resource type',
			icon: Boxes,
			description: 'Filter by resource type'
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
			description: 'Filter by exact resource path'
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
			description: 'Search in resource description'
		},
		value: {
			type: 'string' as const,
			format: 'json' as const,
			label: 'Value subset',
			icon: Braces,
			description: 'Filter by JSON subset match (e.g., {"bucket": "my-bucket"})'
		},
		label: {
			type: 'oneof' as const,
			options: (labels ?? []).map((s) => ({ label: s, value: s })),
			allowNegative: false,
			allowMultiple: true,
			label: 'Label',
			icon: Tag,
			description: 'Filter by label (comma-separated for multiple)'
		},
		...(showUserFoldersFilter
			? {
					user_folders_only: {
						type: 'boolean' as const,
						label: userFoldersLabel || 'User folders only',
						icon: Users,
						description: 'Show only resources in user folders'
					}
				}
			: {})
	} satisfies FilterSchemaRec
}
