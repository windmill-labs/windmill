import { describe, it, expect } from 'vitest'
import { workspaceParamAllowed, workspaceAgnosticRoute } from './workspaceParam'

describe('workspaceParamAllowed', () => {
	it('allows regular workspace-scoped app routes', () => {
		expect(workspaceParamAllowed('/')).toBe(true)
		expect(workspaceParamAllowed('/runs')).toBe(true)
		expect(workspaceParamAllowed('/flows/get/u/me/flow')).toBe(true)
		expect(workspaceParamAllowed('/scripts/edit/u/me/script')).toBe(true)
		expect(workspaceParamAllowed('/apps/get/u/me/app')).toBe(true)
		expect(workspaceParamAllowed('/variables')).toBe(true)
		expect(workspaceParamAllowed('/workspace_settings')).toBe(true)
	})

	it('excludes auth, selection, onboarding and instance routes under /user/', () => {
		expect(workspaceParamAllowed('/user/login')).toBe(false)
		expect(workspaceParamAllowed('/user/logout')).toBe(false)
		expect(workspaceParamAllowed('/user/workspaces')).toBe(false)
		expect(workspaceParamAllowed('/user/accept_invite')).toBe(false)
		expect(workspaceParamAllowed('/user/create_workspace')).toBe(false)
		expect(workspaceParamAllowed('/user/first-time')).toBe(false)
		expect(workspaceParamAllowed('/user/onboarding')).toBe(false)
		expect(workspaceParamAllowed('/user/instance_settings')).toBe(false)
		expect(workspaceParamAllowed('/user/login_callback/google')).toBe(false)
	})

	it('excludes OAuth flows', () => {
		expect(workspaceParamAllowed('/oauth/callback/github')).toBe(false)
		expect(workspaceParamAllowed('/oauth/callback_slack')).toBe(false)
		expect(workspaceParamAllowed('/oauth/mcp_authorize')).toBe(false)
	})

	it('does not exclude routes that merely contain the prefixes mid-path', () => {
		// Defensive: only the leading segment should match.
		expect(workspaceParamAllowed('/apps/get/u/me/user/dashboard')).toBe(true)
		expect(workspaceParamAllowed('/scripts/get/f/team/oauth-helper')).toBe(true)
	})
})

describe('workspaceAgnosticRoute', () => {
	it('flags instance-level routes as workspace-agnostic', () => {
		expect(workspaceAgnosticRoute('/workers')).toBe(true)
		expect(workspaceAgnosticRoute('/service_logs')).toBe(true)
		expect(workspaceAgnosticRoute('/instance_groups')).toBe(true)
		expect(workspaceAgnosticRoute('/concurrency_groups')).toBe(true)
		expect(workspaceAgnosticRoute('/workers/sub/path')).toBe(true)
	})

	it('treats workspace-scoped routes as not agnostic', () => {
		expect(workspaceAgnosticRoute('/')).toBe(false)
		expect(workspaceAgnosticRoute('/runs')).toBe(false)
		expect(workspaceAgnosticRoute('/audit_logs')).toBe(false)
		expect(workspaceAgnosticRoute('/variables')).toBe(false)
		expect(workspaceAgnosticRoute('/scripts/get/u/me/s')).toBe(false)
	})

	it('does not match on a mid-path or prefix-collision segment', () => {
		expect(workspaceAgnosticRoute('/workers_extra')).toBe(false)
		expect(workspaceAgnosticRoute('/apps/get/u/me/workers')).toBe(false)
	})

	it('agnostic routes are still within the allowed (non-hard-excluded) set', () => {
		// They must pass workspaceParamAllowed so the param can be read/adopted
		// before being stripped.
		expect(workspaceParamAllowed('/workers')).toBe(true)
		expect(workspaceParamAllowed('/service_logs')).toBe(true)
	})
})
