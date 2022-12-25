import { browser } from "$app/environment";
import { derived, type Readable, writable } from "svelte/store";
import type { UserWorkspaceList } from "$lib/gen/models/UserWorkspaceList.js";
import { getUserExt } from "./user";
import type { TokenResponse } from "./gen";

export interface UserExt {
  email: string;
  username: string;
  is_admin: boolean;
  is_super_admin: boolean;
  operator: boolean;
  created_at: string;
  groups: string[];
  pgroups: string[];
  folders: string[]
}

let persistedWorkspace = browser && localStorage.getItem("workspace");

export const usageStore = writable<number>(0);
export const oauthStore = writable<TokenResponse | undefined>(undefined);
export const userStore = writable<UserExt | undefined>(undefined);
export const workspaceStore = writable<string | undefined>(
  persistedWorkspace ? String(persistedWorkspace) : undefined,
);
export const starStore = writable(1);
export const usersWorkspaceStore = writable<UserWorkspaceList | undefined>(
  undefined,
);
export const superadmin = writable<String | false | undefined>(undefined);
export const userWorkspaces: Readable<
  Array<{
    id: string;
    name: string;
    username: string;
  }>
> = derived(
  [usersWorkspaceStore, superadmin],
  ([store, superadmin]) => {
    const originalWorkspaces = (store?.workspaces ?? []);
    if (superadmin) {
      return [...originalWorkspaces.filter((x) => x.id != 'starter' && x.id != 'admins'), {
        id: "admins",
        name: "Admins",
        username: "superadmin",
      }, {
        id: "starter",
        name: "Starter",
        username: "superadmin",
      }];
    } else {
      return originalWorkspaces;
    }
  },
);
export const hubScripts = writable<
  | Array<{
    path: string;
    summary: string;
    approved: boolean;
    kind: string;
    app: string;
    ask_id: number;
  }>
  | undefined
>(undefined);

if (browser) {
  workspaceStore.subscribe(async (workspace) => {
    if (workspace) {
      localStorage.setItem("workspace", String(workspace));
      userStore.set(await getUserExt(workspace));
    } else {
      userStore.set(undefined);
    }
  });
}

export function clearStores(): void {
  localStorage.removeItem("workspace");
  userStore.set(undefined);
  workspaceStore.set(undefined);
  usersWorkspaceStore.set(undefined);
  superadmin.set(undefined);
}
