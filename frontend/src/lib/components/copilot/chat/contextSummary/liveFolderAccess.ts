import type { AiSkillListItem } from '../global/core'
import type { McpServer } from '../global/mcpTools'
import type { AccessEntry, AccessScope } from './folderAccess'
import { scopeForWorkspacePath } from '../agentAccessPolicy'
import { AppService, FlowService, ResourceService, ScriptService, VariableService } from '$lib/gen'

function displayName(path: string): string {
	return path.split('/').pop() ?? path
}

function entry(kind: AccessEntry['kind'], path: string): AccessEntry {
	return {
		id: `${kind}:${path}`,
		kind,
		name: displayName(path),
		path,
		description: ''
	}
}

export function buildLiveAccessScopes(
	username: string,
	folders: string[],
	skills: AiSkillListItem[],
	mcpServers: McpServer[],
	items: AccessEntry[] = []
): AccessScope[] {
	const scopes: AccessScope[] = [
		{
			id: 'personal',
			kind: 'personal',
			name: 'Personal',
			description: 'Your items, skills, and personal instructions.',
			entries: []
		},
		{
			id: 'workspace',
			kind: 'workspace',
			name: 'Workspace-wide',
			description: 'Shared context and workspace-level capabilities.',
			entries: []
		},
		...[...new Set(folders)].sort().map(
			(name): AccessScope => ({
				id: `folder:${name}`,
				kind: 'folder',
				name,
				path: `f/${name}`,
				description: `Items and AI context in f/${name}.`,
				entries: []
			})
		)
	]
	const byId = new Map(scopes.map((scope) => [scope.id, scope]))
	for (const item of items) {
		const scope = scopeForWorkspacePath(item.path, username)
		if (scope) byId.get(scope)?.entries.push(item)
	}
	for (const skill of skills) {
		const scope = scopeForWorkspacePath(skill.path, username)
		if (scope) byId.get(scope)?.entries.push(entry('skill', skill.path))
	}
	for (const server of mcpServers) {
		const scope = scopeForWorkspacePath(server.path, username)
		if (scope) byId.get(scope)?.entries.push(entry('mcp', server.path))
	}
	for (const scope of scopes) scope.entries.sort((a, b) => a.name.localeCompare(b.name))
	return scopes
}

export async function loadLiveAccessItems(workspace: string): Promise<AccessEntry[]> {
	if (!workspace) return []
	const [scripts, flows, apps, resources, variables] = await Promise.all([
		ScriptService.listScripts({ workspace, perPage: 1000, withoutDescription: true }),
		FlowService.listFlows({ workspace, perPage: 1000, withoutDescription: true }),
		AppService.listApps({ workspace, perPage: 1000 }),
		ResourceService.listResource({ workspace, perPage: 1000 }),
		VariableService.listVariable({ workspace, perPage: 1000 })
	])
	return [
		...scripts.map((item) => entry('script', item.path)),
		...flows.map((item) => entry('flow', item.path)),
		...apps.map((item) => entry('app', item.path)),
		...resources
			.filter((item) => item.resource_type !== 'ai_skill' && item.resource_type !== 'mcp')
			.map((item) => entry('resource', item.path)),
		...variables.map((item) => entry('variable', item.path))
	]
}
