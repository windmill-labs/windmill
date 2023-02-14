import { get } from "svelte/store";
import { type User, UserService } from "$lib/gen";
import { superadmin, type UserExt } from "./stores.js";
import { goto } from "$app/navigation";

export async function refreshSuperadmin(): Promise<void> {
  if (get(superadmin) == undefined) {
    try {
      const me = await UserService.globalWhoami();
      if (me.super_admin) {
        superadmin.set(me.email);
      } else {
        superadmin.set(false);
      }
    } catch {
      superadmin.set(false);
      goto("/user/logout");
    }
  }
}

export async function getUserExt(
  workspace: string,
): Promise<UserExt | undefined> {
  try {
    const user = await UserService.whoami({ workspace });
    return mapUserToUserExt(user);
  } catch (error) {
    return undefined;
  }
}

function mapUserToUserExt(user: User): UserExt {
  return {
    ...user,
    groups: user.groups!,
    pgroups: user.groups!.map((x) => `g/${x}`),
  };
}
