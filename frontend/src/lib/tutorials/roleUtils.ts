import type { UserExt } from '$lib/stores'

export type Role = 'admin' | 'developer' | 'operator'

/**
 * Get the effective role of a user based on their database flags.
 * - Admin: user.is_admin === true
 * - Operator: user.operator === true (and not admin)
 * - Developer: default (neither admin nor operator)
 */
export function getUserEffectiveRole(user: UserExt | null | undefined): Role | null {
	if (!user) return null
	if (user.is_admin) return 'admin'
	if (user.operator) return 'operator'
	return 'developer'
}

/**
 * Check if a role has access to a required role.
 * This is the core role-checking logic used by both normal and preview modes.
 */
function checkRoleMatch(
	userRole: Role,
	requiredRole: Role
): boolean {
	if (requiredRole === 'admin') return userRole === 'admin'
	if (requiredRole === 'operator') return userRole === 'operator' || userRole === 'admin'
	if (requiredRole === 'developer') return userRole === 'developer' || userRole === 'admin'
	return false
}

/**
 * Check if a user or preview role has access based on a roles array.
 * This is the unified function that handles both normal user access and admin preview mode.
 */
export function hasRoleAccess(
	user: UserExt | null | undefined,
	roles?: Role[],
	previewRole?: Role
): boolean {
	// No roles specified = available to everyone
	if (!roles || roles.length === 0) return true
	
	// If previewRole is provided, use it (admin preview mode)
	// Otherwise, derive role from user
	const effectiveRole = previewRole ?? getUserEffectiveRole(user)
	if (!effectiveRole) return false

	// Check if effective role has any of the required roles
	return roles.some((role) => checkRoleMatch(effectiveRole, role))
}

/**
 * Check if a preview role has access based on a roles array.
 * Used by admins to preview what other roles can see.
 * Uses exact role matching - only shows tutorials explicitly marked for the preview role.
 */
export function hasRoleAccessForPreview(
	previewRole: Role,
	roles?: Role[]
): boolean {
	// No roles specified = available to everyone
	if (!roles || roles.length === 0) return true

	// Exact role match - tutorial must explicitly include the preview role
	return roles.includes(previewRole)
}

