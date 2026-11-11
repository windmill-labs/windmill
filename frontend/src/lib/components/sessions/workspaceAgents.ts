import { DraftService, ResourceService } from '$lib/gen'

export type WorkspaceAgent = {
	path: string
	summary: string
	draftOnly: boolean
	hasDraft: boolean
}

/** The workspace's agents as a session shows them: the deployed ones, then those that exist only
 * as a draft. */
export async function listWorkspaceAgents(workspace: string): Promise<WorkspaceAgent[]> {
	const [deployed, drafts] = await Promise.all([
		ResourceService.listResource({ workspace, resourceType: 'ai_agent', perPage: 1000 }),
		// Without drafts the deployed agents still list, unmarked.
		DraftService.listDrafts({ workspace }).catch((error) => {
			console.error('Failed to list drafts for the workspace agents', error)
			return []
		})
	])
	const agentDrafts = drafts.filter((d) => d.kind === 'resource' && d.resource_type === 'ai_agent')
	const drafted = new Set(agentDrafts.map((d) => d.path))
	return [
		...deployed.map((r) => ({
			path: r.path,
			summary: r.description ?? '',
			draftOnly: false,
			hasDraft: drafted.has(r.path)
		})),
		...agentDrafts
			.filter((d) => d.draft_only)
			.map((d) => ({ path: d.path, summary: d.summary ?? '', draftOnly: true, hasDraft: true }))
	]
}
