import type { User } from '$lib/gen/types.gen'
import type { UserExt } from './stores'

export function mapUserToUserExt(user: User, workspace: string): UserExt {
	const ext: UserExt = {
		...user,
		workspace_id: workspace,
		groups: user.groups!,
		pgroups: user.groups!.map((x) => `g/${x}`)
	}
	if (ext.is_service_account && sessionStorage.getItem('pre_impersonation_token')) {
		ext.impersonating_email = sessionStorage.getItem('pre_impersonation_email') ?? undefined
	}
	return ext
}
