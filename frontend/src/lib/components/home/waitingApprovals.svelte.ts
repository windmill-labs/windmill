import { JobService, type ListWaitingApprovalsResponse } from '$lib/gen'
import { IsDocumentVisible, useInterval } from 'runed'
import { persistedResource } from '$lib/persistedResource.svelte'

export type WaitingApproval = ListWaitingApprovalsResponse['approvals'][number]

/** An approval waits on a person, who acts within minutes, not seconds; Home stays open in many
 * tabs. */
const REFRESH_MS = 60000

/**
 * The runs waiting for an approval the user can give, read from `listWaitingApprovals`. The
 * header status and the side section read the same instance, so their counts agree.
 */
export function useWaitingApprovals(workspace: () => string | undefined) {
	const res = persistedResource(
		() => ({ workspace: workspace() }),
		async ({ workspace }) =>
			workspace ? await JobService.listWaitingApprovals({ workspace }) : undefined,
		{
			name: 'waiting_approvals',
			version: 1,
			scope: ({ workspace }) => (workspace ? { workspace, args: {} } : undefined)
		}
	)
	let now = $state(Date.now())
	const visible = new IsDocumentVisible()
	useInterval(REFRESH_MS, {
		callback: () => {
			if (!visible.current) return
			now = Date.now()
			if (!res.loading) res.refetch()
		}
	})
	return {
		get approvals(): WaitingApproval[] {
			return res.current?.approvals ?? []
		},
		/** All the runs waiting, of which `approvals` holds the most recent. */
		get total(): number {
			return res.current?.total ?? 0
		},
		get now() {
			return now
		}
	}
}

export type WaitingApprovalsState = ReturnType<typeof useWaitingApprovals>

/** The approval page, which shows an approver that one run even without read access to it. */
export function approvalHref(a: WaitingApproval, workspace: string | undefined) {
	return `/approve/${workspace ?? ''}/${a.job_id}`
}

export function approvalTitle(a: WaitingApproval) {
	return a.flow_summary || a.flow_path?.split('/').pop() || 'Flow'
}

export function approvalDetail(a: WaitingApproval) {
	return [
		a.step_summary,
		`started by ${a.started_by}`,
		a.required_approvals > 1 ? `${a.approvals} of ${a.required_approvals} approvals` : undefined
	]
		.filter(Boolean)
		.join(' · ')
}

export function approvalAge(a: WaitingApproval, now: number) {
	const s = Math.max(0, (now - new Date(a.created_at).getTime()) / 1000)
	if (s < 60) return 'now'
	if (s < 3600) return `${Math.floor(s / 60)}m`
	if (s < 86400) return `${Math.floor(s / 3600)}h`
	return `${Math.floor(s / 86400)}d`
}
