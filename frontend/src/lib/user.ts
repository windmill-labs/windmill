import { type User, UserService } from '$lib/gen'
import type { UserExt } from './stores.js'

export async function getUserExt(workspace: string): Promise<UserExt | undefined> {
	try {
		const user = await UserService.whoami({ workspace })
		return mapUserToUserExt(user)
	} catch (error) {
		return undefined
	}
}

function mapUserToUserExt(user: User): UserExt {
	const ext: UserExt = {
		...user,
		groups: user.groups!,
		pgroups: user.groups!.map((x) => `g/${x}`)
	}
	if (ext.is_service_account && sessionStorage.getItem('pre_impersonation_token')) {
		ext.impersonating_email = sessionStorage.getItem('pre_impersonation_email') ?? undefined
	}
	return ext
}
