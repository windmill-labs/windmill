import {
	WorkspaceService,
	type AuthorLevel,
	type EffectiveRole,
	type RoleCapabilities,
	type RoleCapabilitiesPatch,
	type RoleDefinition,
	type RoleVisibility,
	type WorkspaceRoles
} from '$lib/gen'

/** Not a role: the `is_admin` flag, offered next to the roles wherever a member's access is picked. */
export const ADMIN_ROLE = 'admin'

export const DEVELOPER_ROLE = 'developer'
export const OPERATOR_ROLE = 'operator'

export const VISIBILITY_KEYS: { key: keyof RoleVisibility; label: string }[] = [
	{ key: 'runs', label: 'Runs' },
	{ key: 'schedules', label: 'Schedules' },
	{ key: 'resources', label: 'Resources' },
	{ key: 'variables', label: 'Variables' },
	{ key: 'assets', label: 'Assets' },
	{ key: 'triggers', label: 'Triggers' },
	{ key: 'audit_logs', label: 'Audit logs' },
	{ key: 'groups', label: 'Groups' },
	{ key: 'folders', label: 'Folders' },
	{ key: 'workers', label: 'Workers' }
]

const ALL_VISIBLE: RoleVisibility = {
	runs: true,
	schedules: true,
	resources: true,
	variables: true,
	assets: true,
	triggers: true,
	audit_logs: true,
	groups: true,
	folders: true,
	workers: true
}

// Mirror `RoleCapabilities::FULL` and `RoleCapabilities::OPERATOR` in `windmill_common::roles`.
export const FULL_CAPABILITIES: RoleCapabilities = {
	code: true,
	flows: 'full',
	apps: 'full',
	manage_schedules: true,
	manage_triggers: true,
	visibility: ALL_VISIBLE
}

const OPERATOR_CAPABILITIES: RoleCapabilities = {
	code: false,
	flows: 'none',
	apps: 'none',
	manage_schedules: true,
	manage_triggers: true,
	visibility: ALL_VISIBLE
}

export const DEFAULT_INSTANCE_ROLES: RoleDefinition[] = [
	{ name: DEVELOPER_ROLE, label: 'Developer', capabilities: {} },
	{ name: OPERATOR_ROLE, label: 'Operator', capabilities: {} }
]

/** What a role named `name` holds before any configuration: `RoleCapabilities::base_for`. */
export function baseCapabilitiesFor(name: string): RoleCapabilities {
	return applyCapabilitiesPatch(
		name === DEVELOPER_ROLE ? FULL_CAPABILITIES : OPERATOR_CAPABILITIES,
		undefined
	)
}

export function applyCapabilitiesPatch(
	base: RoleCapabilities,
	patch: RoleCapabilitiesPatch | undefined
): RoleCapabilities {
	// Spread rather than `structuredClone`: `base` may be a Svelte state proxy, which cannot be cloned.
	const out: RoleCapabilities = { ...base, visibility: { ...base.visibility } }
	if (!patch) return out
	if (patch.code !== undefined) out.code = patch.code
	if (patch.flows !== undefined) out.flows = patch.flows
	if (patch.apps !== undefined) out.apps = patch.apps
	if (patch.manage_schedules !== undefined) out.manage_schedules = patch.manage_schedules
	if (patch.manage_triggers !== undefined) out.manage_triggers = patch.manage_triggers
	for (const { key } of VISIBILITY_KEYS) {
		const v = patch.visibility?.[key]
		if (v !== undefined) out.visibility[key] = v
	}
	return out
}

/** The error the server raises for these capabilities, if any (`RoleCapabilities::validate`). */
export function capabilitiesError(caps: RoleCapabilities): string | undefined {
	if (!caps.code && (caps.flows === 'full' || caps.apps === 'full')) {
		return 'Full flow or app authoring requires the code capability'
	}
	return undefined
}

/** `validate_role_name` in `windmill_common::roles`. */
export function roleNameError(name: string): string | undefined {
	if (name === ADMIN_ROLE) return 'The name admin is reserved'
	if (!/^[a-z0-9_-]{1,50}$/.test(name)) {
		return 'Use 1 to 50 lowercase letters, digits, underscores or hyphens'
	}
	return undefined
}

/** A patch with no key set, so it is dropped rather than stored as `{}`. */
export function isEmptyPatch(patch: RoleCapabilitiesPatch | undefined): boolean {
	if (!patch) return true
	const { visibility, ...rest } = patch
	return (
		Object.values(rest).every((v) => v === undefined) &&
		Object.values(visibility ?? {}).every((v) => v === undefined)
	)
}

/**
 * What `parse_instance_roles` would refuse in a `workspace_roles` value, keyed `<index>.name`,
 * `<index>.capabilities` or `list`. An unset value is the defaults and always valid.
 */
export function workspaceRolesErrors(value: unknown): Record<string, string> {
	if (value == undefined) return {}
	if (!Array.isArray(value)) return { list: 'Expected a list of roles' }
	if (value.length === 0) return { list: 'Keep at least one role' }
	const errors: Record<string, string> = {}
	const seen = new Set<string>()
	value.forEach((r: RoleDefinition, i) => {
		const nameError = roleNameError(r?.name ?? '')
		if (nameError) errors[`${i}.name`] = nameError
		else if (seen.has(r.name)) errors[`${i}.name`] = 'Another role has this name'
		seen.add(r?.name)
		const capsError = capabilitiesError(
			applyCapabilitiesPatch(baseCapabilitiesFor(r?.name ?? ''), r?.capabilities)
		)
		if (capsError) errors[`${i}.capabilities`] = capsError
	})
	return errors
}

/** Whether `roles` is the default list, which is stored as no value at all. */
export function isDefaultInstanceRoles(roles: RoleDefinition[]): boolean {
	return (
		roles.length === DEFAULT_INSTANCE_ROLES.length &&
		roles.every(
			(r, i) =>
				r.name === DEFAULT_INSTANCE_ROLES[i].name &&
				(r.label ?? '') === DEFAULT_INSTANCE_ROLES[i].label &&
				isEmptyPatch(r.capabilities)
		)
	)
}

export function authorLevelLabel(level: AuthorLevel): string {
	return level === 'none' ? 'None' : level === 'compose' ? 'Compose' : 'Full'
}

export async function loadWorkspaceRoles(workspace: string): Promise<WorkspaceRoles> {
	return await WorkspaceService.listWorkspaceRoles({ workspace })
}

/** Role picker items: the workspace's roles in their order, then Admin. */
export function roleItems(
	roles: EffectiveRole[] | undefined,
	opts: { admin?: boolean } = {}
): { label: string; value: string }[] {
	const items = (roles ?? []).map((r) => ({ label: r.label, value: r.name }))
	if (opts.admin ?? true) items.push({ label: 'Admin', value: ADMIN_ROLE })
	return items
}

export function roleLabel(roles: EffectiveRole[] | undefined, name: string | undefined): string {
	if (!name) return ''
	if (name === ADMIN_ROLE) return 'Admin'
	return roles?.find((r) => r.name === name)?.label ?? name
}

/** The picker value of a member, invite or mapping: `admin` for an admin, its role otherwise. */
export function pickerValue(m: { is_admin?: boolean; role?: string; operator?: boolean }): string {
	if (m.is_admin) return ADMIN_ROLE
	return m.role ?? (m.operator ? OPERATOR_ROLE : DEVELOPER_ROLE)
}

/**
 * The fields a picked value is sent as. An admin keeps a role underneath the flag, `developer`
 * being what the server gives one when none is named; `operator` is set for servers that predate
 * roles.
 */
export function roleRequestFields(value: string): {
	is_admin: boolean
	operator: boolean
	role?: string
} {
	if (value === ADMIN_ROLE) return { is_admin: true, operator: false }
	return { is_admin: false, operator: value === OPERATOR_ROLE, role: value }
}
