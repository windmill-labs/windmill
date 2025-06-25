import type { AssetKind } from '$lib/gen'

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
