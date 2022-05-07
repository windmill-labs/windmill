import { writable, derived, readable } from 'svelte/store';
export const userStore = writable(undefined);
export const workspaceStore = writable(undefined);
export const usersWorkspaceStore = writable(undefined);
export const usernameStore = derived([usersWorkspaceStore, workspaceStore], ($values, set) => {
	set($values[0]?.workspaces.find((x) => x.id == $values[1])?.username);
});
export const superadmin = writable(undefined);
//# sourceMappingURL=stores.js.map
