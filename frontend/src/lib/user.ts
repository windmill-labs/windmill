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
	return {
		...user,
		groups: user.groups!,
		pgroups: user.groups!.map((x) => `g/${x}`)
	}
}
