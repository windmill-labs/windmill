import { base } from '$lib/base'

type ItemType = 'script' | 'flow' | 'app' | 'raw_app'

export function buildForkEditUrl(itemType: ItemType, itemPath: string): string {
	let editPath: string
	switch (itemType) {
		case 'script':
			editPath = `${base}/scripts/edit/${itemPath}`
			break
		case 'flow':
			editPath = `${base}/flows/edit/${itemPath}?nodraft=true`
			break
		case 'app':
			editPath = `${base}/apps/edit/${itemPath}?nodraft=true`
			break
		case 'raw_app':
			editPath = `${base}/apps_raw/edit/${itemPath}?nodraft=true`
			break
	}
	return `${base}/user/fork_workspace?rd=${encodeURIComponent(editPath)}`
}
