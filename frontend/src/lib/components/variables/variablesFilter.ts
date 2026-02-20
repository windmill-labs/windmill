import { FileCode, FileText, FolderIcon, Key } from 'lucide-svelte'
import type { FilterSchemaRec } from '../FilterSearchbar.svelte'

export function buildVariablesFilterSchema({
	paths,
	owners
}: {
	paths: string[]
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
			description: 'Filter by exact variable path'
		},
		path_start: {
			type: 'string' as const,
			label: 'Path prefix',
			icon: FolderIcon,
			description: 'Filter by path prefix (e.g., "f/folder/")'
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
			description: 'Search in variable description'
		},
		value: {
			type: 'string' as const,
			label: 'Value',
			icon: Key,
			description: 'Search in non-secret variable values'
		}
	} satisfies FilterSchemaRec
}
