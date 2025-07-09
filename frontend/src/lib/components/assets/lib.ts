import type {
	AssetKind as _AssetKind,
	Asset as _Asset,
	ListAssetsResponse,
	AssetUsageAccessType
} from '$lib/gen'

export type Asset = _Asset
export type AssetKind = _AssetKind
export type AssetWithAccessType = Asset & { access_type?: AssetUsageAccessType }

export function formatAsset(asset: Asset): string {
	switch (asset.kind) {
		case 'resource':
			return `res://${asset.path}`
		case 's3object':
			return `s3://${asset.path}`
		case 'variable':
			return `var://${asset.path}`
	}
}

export function getAssetUsagePageUri(usage: ListAssetsResponse[number]['usages'][number]) {
	if (usage.kind === 'script') {
		return `/scripts/get/${usage.path}`
	} else if (usage.kind === 'flow') {
		return `/flows/get/${usage.path}`
	}
}

export function assetEq(a: Asset | undefined, b: Asset | undefined): boolean {
	if (!a || !b) return a === b
	return a.kind === b.kind && a.path === b.path
}

export function parseAssetFromString(s: string): Asset | undefined {
	if (s.startsWith('res://')) {
		return { kind: 'resource', path: s.slice(6) }
	} else if (s.startsWith('$res:')) {
		return { kind: 'resource', path: s.slice(5) }
	} else if (s.startsWith('s3://')) {
		return { kind: 's3object', path: s.slice(5) }
	} else if (s.startsWith('var://')) {
		return { kind: 'variable', path: s.slice(6) }
	}
	return undefined
}

export function formatAssetKind(kind: AssetKind): string {
	switch (kind) {
		case 'resource':
			return 'Resource'
		case 's3object':
			return 'S3 Object'
		case 'variable':
			return 'Variable'
	}
}
