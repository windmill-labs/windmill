import { describe, it, expect } from 'vitest'
import {
	buildWorkspaceSettingsUrl,
	buildAuditLogsUrl,
	buildResourcesUrl,
	buildVariablesUrl
} from './pageNavigation'

function parse(appPath: string): URL {
	return new URL(appPath, 'http://x')
}

describe('pageNavigation builders', () => {
	it('opens workspace settings on a specific tab', () => {
		const u = parse(buildWorkspaceSettingsUrl({ tab: 'git_sync' }))
		expect(u.pathname).toBe('/workspace_settings')
		expect(u.searchParams.get('tab')).toBe('git_sync')
	})

	it('opens workspace settings with no tab', () => {
		const u = parse(buildWorkspaceSettingsUrl({}))
		expect(u.pathname).toBe('/workspace_settings')
		expect(u.search).toBe('')
	})

	it('audit logs allow-lists its own keys and drops others', () => {
		const u = parse(
			buildAuditLogsUrl({ username: 'admin', operation: 'jobs.run', status: 'failure' })
		)
		expect(u.pathname).toBe('/audit_logs')
		expect(u.searchParams.get('username')).toBe('admin')
		expect(u.searchParams.get('operation')).toBe('jobs.run')
		expect(u.searchParams.has('status')).toBe(false) // not an audit-logs key
	})

	it('resources keeps resource_type + path, drops runs-only keys', () => {
		const u = parse(
			buildResourcesUrl({ resource_type: 'postgres', path: 'f/x', status: 'failure' })
		)
		expect(u.searchParams.get('resource_type')).toBe('postgres')
		expect(u.searchParams.get('path')).toBe('f/x')
		expect(u.searchParams.has('status')).toBe(false)
	})

	it('variables keeps path + owner only', () => {
		const u = parse(buildVariablesUrl({ path: 'f/x', owner: 'u/alice', resource_type: 'postgres' }))
		expect(u.searchParams.get('path')).toBe('f/x')
		expect(u.searchParams.get('owner')).toBe('u/alice')
		expect(u.searchParams.has('resource_type')).toBe(false)
	})
})
