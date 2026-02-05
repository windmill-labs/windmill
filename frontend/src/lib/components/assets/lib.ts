import type {
	AssetKind as _AssetKind,
	Asset as _Asset,
	ListAssetsResponse,
	AssetUsageAccessType,
	FlowModule,
	ScriptArgs
} from '$lib/gen'
import { capitalize } from '$lib/utils'
export type Asset = _Asset
export type AssetKind = _AssetKind
export type AssetWithAccessType = Asset & { access_type?: AssetUsageAccessType }
export type AssetWithAltAccessType = AssetWithAccessType & {
	alt_access_type?: AssetUsageAccessType
	columns?: Record<string, AssetUsageAccessType>
}
export type AssetUsage = ListAssetsResponse['assets'][number]['usages'][number]

export function formatAsset(asset: Asset): string {
	switch (asset.kind) {
		case 'resource':
			return `res://${asset.path}`
		case 's3object':
			return `s3://${asset.path}`
		case 'ducklake':
			return `ducklake://${asset.path}`
		case 'datatable':
			return `datatable://${asset.path}`
	}
	return 'unknown'
}

export function formatShortAssetPath(asset: Asset): string {
	if (asset.kind === 'datatable' && asset.path === 'main') return 'Main data table'
	if (asset.kind === 'ducklake' && asset.path === 'main') return 'Main ducklake'
	const s = asset.path.split('/').pop() || asset.path
	if (s.includes('?table=')) return s.split('?table=')[1]
	return s
}

export function getAssetUsagePageUri(usage: AssetUsage) {
	if (usage.kind === 'script') {
		return `/scripts/get/${usage.path}`
	} else if (usage.kind === 'flow') {
		return `/flows/get/${usage.path}`
	} else if (usage.kind === 'job') {
		return `/run/${usage.path}`
	}
}

export function assetEq(a: Asset | undefined, b: Asset | undefined): boolean {
	if (!a || !b) return a === b
	return a.kind === b.kind && a.path === b.path
}

export function assetsEq(a: Asset[], b: Asset[]): boolean {
	if (a.length !== b.length) return false
	return a.every((asset, index) => assetEq(asset, b[index]))
}

export function parseAssetFromString(s: string): Asset | undefined {
	if (s.startsWith('res://')) {
		return { kind: 'resource', path: s.slice(6) }
	} else if (s.startsWith('$res:')) {
		return { kind: 'resource', path: s.slice(5) }
	} else if (s.startsWith('s3://')) {
		return { kind: 's3object', path: s.slice(5) }
	}
	return undefined
}

export function formatAssetKind(asset: {
	kind: AssetKind
	metadata?: { resource_type?: string }
}): string {
	switch (asset.kind) {
		case 'resource':
			if (asset.metadata?.resource_type) {
				if (asset.metadata.resource_type === 'state') return 'State'
				if (asset.metadata.resource_type === 'cache') return 'Cache'
				if (asset.metadata.resource_type === 'app_theme') return 'App Theme'
				return `${capitalize(asset.metadata.resource_type)} resource`
			} else {
				return 'metadata' in asset ? 'Invalid resource' : 'Resource'
			}
		case 's3object':
			return 'S3 Object'
		case 'ducklake':
			return 'Ducklake'
		case 'datatable':
			return 'Data table'
	}
}

export function formatAssetAccessType(accessType: AssetUsageAccessType | undefined) {
	switch (accessType) {
		case 'r':
			return 'Read'
		case 'w':
			return 'Write'
		case 'rw':
			return 'R/W'
	}
}

export function getAccessType(asset: AssetWithAltAccessType): AssetUsageAccessType | undefined {
	if (asset.access_type) return asset.access_type
	if (asset.alt_access_type) return asset.alt_access_type
}

export function getFlowModuleAssets(
	flowModuleValue: FlowModule,
	additionalAssetsMap?: Record<string, AssetWithAccessType[]>
): AssetWithAccessType[] | undefined {
	if (flowModuleValue.value.type === 'rawscript') return flowModuleValue.value.assets
	if (flowModuleValue.value.type === 'script' || flowModuleValue.value.type === 'flow') {
		const additionalAssets = additionalAssetsMap?.[flowModuleValue.id]
		if (additionalAssets) return additionalAssets
	}
	return undefined
}

export function parseInputArgsAssets(args: ScriptArgs): AssetWithAccessType[] {
	const arr: AssetWithAccessType[] = []
	for (const v of Object.values(args)) {
		if (typeof v === 'string') {
			const asset = parseAssetFromString(v)
			if (asset) arr.push(asset)
		} else if (v && typeof v === 'object' && typeof v['s3'] === 'string') {
			const s3 = v['s3']
			const storage = typeof v['storage'] == 'string' ? v['storage'] : undefined
			arr.push({ kind: 's3object', path: `${storage ?? ''}/${s3}` })
		}
	}
	return arr
}
