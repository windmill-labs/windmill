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
