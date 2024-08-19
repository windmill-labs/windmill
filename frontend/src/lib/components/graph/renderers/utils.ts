import type { FlowStatusModule } from '$lib/gen'
import type { GraphModuleState } from '../model'

export function getStraightLinePath({ sourceX, sourceY, targetY }) {
	return `M${sourceX},${sourceY} L${sourceX},${targetY - 100}`
}

export function computeBorderStatus(
	branchIndex: number,
	type: 'branchone' | 'branchall',
	graphModuleState: GraphModuleState | undefined
): FlowStatusModule['type'] | undefined {
	if (type === 'branchone') {
		const branchChosen = graphModuleState?.branchChosen

		if (branchChosen === branchIndex) {
			return graphModuleState?.type
		}
	} else {
		let flow_jobs_success = graphModuleState?.flow_jobs_success
		if (!flow_jobs_success) {
			return 'WaitingForPriorSteps'
		} else {
			let status = flow_jobs_success?.[branchIndex]
			if (status == undefined) {
				return 'WaitingForExecutor'
			} else {
				return status ? 'Success' : 'Failure'
			}
		}
	}
}
