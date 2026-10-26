import { DraftService, ResourceService } from '$lib/gen'

export type WorkspaceAgent = { path: string; summary: string; draftOnly: boolean }

/** The workspace's agents as a session shows them: the deployed ones, then those that exist only
 * as a draft. The draft listing does not say which resources are agents, so those are asked. */
export async function listWorkspaceAgents(workspace: string): Promise<WorkspaceAgent[]> {
	const [deployed, drafts] = await Promise.all([
		ResourceService.listResource({ workspace, resourceType: 'ai_agent', perPage: 1000 }),
		DraftService.listDrafts({ workspace }).catch(() => [])
	])
	const draftOnly = await Promise.all(
		drafts
			.filter((d) => d.kind === 'resource' && d.draft_only)
			.map((d) =>
				ResourceService.getResource({ workspace, path: d.path, getDraft: true }).then(
					(r) =>
						r.resource_type === 'ai_agent'
							? { path: d.path, summary: r.description ?? '', draftOnly: true }
							: undefined,
					() => undefined
				)
			)
	)
	return [
		...deployed.map((r) => ({ path: r.path, summary: r.description ?? '', draftOnly: false })),
		...draftOnly.filter((a) => a !== undefined)
	]
}
