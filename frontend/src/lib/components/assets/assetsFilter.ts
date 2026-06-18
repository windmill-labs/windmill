import { FileCode, FolderIcon, Box, Braces } from 'lucide-svelte'
import type { FilterSchemaRec } from '../FilterSearchbar.svelte'

export function buildAssetsFilterSchema({
	paths,
	assetKinds
}: {
	paths: string[]
	assetKinds: string[]
}) {
	return {
		_default_: {
			type: 'string' as const,
			hidden: true
		},
		asset_path: {
			type: 'string' as const,
			label: 'Asset path pattern',
			icon: FolderIcon,
			description: 'Filter by asset path pattern (case-insensitive)'
		},
		asset_kinds: {
			type: 'oneof' as const,
			options: assetKinds.map((s) => ({ label: s, value: s })),
			allowCustomValue: false,
			allowNegative: false,
			allowMultiple: true,
			label: 'Asset kind',
			icon: Box,
			description: 'Filter by asset kind (s3object, resource, variable, etc.)'
		},
		usage_path: {
			type: 'string' as const,
			label: 'Usage path pattern',
			icon: FileCode,
			description: 'Filter by usage path pattern (case-insensitive)'
		},
		path: {
			type: 'oneof' as const,
			options: paths.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			allowNegative: false,
			allowMultiple: false,
			label: 'Asset path',
			icon: FileCode,
			description: 'Filter by exact asset path'
		},
		columns: {
			type: 'string' as const,
			label: 'Columns',
			icon: Braces,
			description: 'Filter by comma-separated column names (e.g., col1,col2,col3)'
		}
	} satisfies FilterSchemaRec
}
