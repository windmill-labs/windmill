import type { FlowStatusModule } from '$lib/gen'
import type { GraphModuleState } from '../model'

export function getStraightLinePath({ sourceX, sourceY, targetY }) {
	return `M${sourceX},${sourceY} L${sourceX},${targetY - 100}`
}

export function computeBorderStatus(
	branchIndex: number,
	graphModuleState: GraphModuleState | undefined
): FlowStatusModule['type'] | undefined {
	const flowJobSuccess = graphModuleState?.flow_jobs_success

	if (!Array.isArray(flowJobSuccess)) {
		return 'WaitingForPriorSteps'
	}

	const jobStatus = flowJobSuccess[branchIndex]

	if (jobStatus === undefined) {
		return 'WaitingForExecutor'
	}

	return jobStatus ? 'Success' : 'Failure'
}
