import { Boxes, FileCode, FileText, FolderIcon, Braces } from 'lucide-svelte'
import type { FilterSchemaRec } from '../FilterSearchbar.svelte'

export function buildResourcesFilterSchema({
	paths,
	resourceTypes,
	owners
}: {
	paths: string[]
	resourceTypes: string[]
	owners: string[]
}) {
	return {
		path: {
			type: 'oneof' as const,
			options: paths.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
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
		resource_type: {
			type: 'oneof' as const,
			options: resourceTypes.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			allowNegative: false,
			allowMultiple: true,
			label: 'Resource type',
			icon: Boxes,
			description: 'Filter by resource type'
		},
		owner: {
			type: 'oneof' as const,
			options: owners.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			allowNegative: false,
			allowMultiple: false,
			label: 'Owner',
			icon: FolderIcon,
			description: 'Filter by owner (folder or user path)'
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
		}
	} satisfies FilterSchemaRec
}
