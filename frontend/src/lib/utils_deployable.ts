import { minimatch } from 'minimatch'
import type { WorkspaceDeployUISettings } from './gen'

type DeployUIType = 'script' | 'flow' | 'app' | 'resource' | 'variable' | 'secret'

export function isDeployable(
	type: DeployUIType,
	path: string,
	deployUiSettings: WorkspaceDeployUISettings | undefined
) {
	if (deployUiSettings == undefined) {
		return false
	}

	if (deployUiSettings.include_type != undefined && !deployUiSettings.include_type.includes(type)) {
		return false
	}

	if (
		deployUiSettings.include_path != undefined &&
		deployUiSettings.include_path.length != 0 &&
		deployUiSettings.include_path.every((x) => !minimatch(path, x))
	) {
		return false
	}

	return true
}

export const ALL_DEPLOYABLE: WorkspaceDeployUISettings = {
	include_path: [],
	include_type: ['script', 'flow', 'app', 'resource', 'variable', 'secret']
}
