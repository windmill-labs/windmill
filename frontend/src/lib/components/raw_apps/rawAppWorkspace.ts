import { getContext, setContext } from 'svelte'

// The workspace a raw-app editor operates on. In a session preview this is the
// session's acting workspace, which differs from the navigation `$workspaceStore`
// (a session deliberately leaves the nav store on the workspace the top nav
// points at). RawAppEditor provides it once; the sidebar sub-components (inline
// scripts, datatable/shared-UI drawers, DB selector, …) read it so their lookups
// target the workspace the app actually lives in rather than the nav workspace.
//
// A getter (not a value) so the live `$derived` opWorkspace is read reactively at
// each call site. Consumers fall back to `$workspaceStore` when unset — e.g. the
// full-page app editor, where the nav workspace IS the operating workspace.

const KEY = 'RawAppOperatingWorkspace'

export function setRawAppOperatingWorkspace(get: () => string | undefined): void {
	setContext(KEY, get)
}

export function getRawAppOperatingWorkspace(): (() => string | undefined) | undefined {
	return getContext(KEY)
}
