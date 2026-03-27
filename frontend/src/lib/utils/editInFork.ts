import { base } from '$lib/base'
import { getWsBase } from '$lib/workspaceUrl'

type ItemType = 'script' | 'flow' | 'app' | 'raw_app'

export function buildForkEditUrl(itemType: ItemType, itemPath: string): string {
	const wsBase = getWsBase()
	let editPath: string
	switch (itemType) {
		case 'script':
			editPath = `${wsBase}/scripts/edit/${itemPath}`
			break
		case 'flow':
			editPath = `${wsBase}/flows/edit/${itemPath}?nodraft=true`
			break
		case 'app':
			editPath = `${wsBase}/apps/edit/${itemPath}?nodraft=true`
			break
		case 'raw_app':
			editPath = `${wsBase}/apps_raw/edit/${itemPath}?nodraft=true`
			break
	}
	return `${base}/user/fork_workspace?rd=${encodeURIComponent(editPath)}`
}
