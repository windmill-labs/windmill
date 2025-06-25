import type { AssetKind, ListAssetsResponse } from '$lib/gen'

export type Asset = {
	path: string
	kind: AssetKind
}

export function parseAsset(asset: string): Asset | undefined {
	if (asset.startsWith('$res:')) return { path: asset.substring(5), kind: 'resource' }
	if (asset.startsWith('s3://')) return { path: asset.substring(5), kind: 's3object' }
}

export function formatAsset(asset: Asset): string {
	switch (asset.kind) {
		case 'resource':
			return `$res:${asset.path}`
		case 's3object':
			return `s3://${asset.path}`
	}
}

export function getAssetUsagePageUri(usage: ListAssetsResponse[number]['usages'][number]) {
	if (usage.usage_kind === 'script') {
		return `/scripts/edit/${usage.usage_path}`
	} else if (usage.usage_kind === 'flow') {
		return `/flows/edit/${usage.usage_path}`
	}
}
