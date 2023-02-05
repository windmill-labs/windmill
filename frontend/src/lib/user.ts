import { get } from 'svelte/store'
import { UserService, type User } from '$lib/gen'
import { superadmin, type UserExt } from './stores.js'

export async function refreshSuperadmin(): Promise<void> {
    if (get(superadmin) == undefined) {
        await UserService.globalWhoami().then((x) => {
            if (x.super_admin) {
                superadmin.set(x.email)
            } else {
                superadmin.set(false)
            }
        })
    }
}

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
