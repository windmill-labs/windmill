// Keyboard-navigation helpers for the fork-diff tree (ForkDiffDrawer).
//
// The tree groups diffs by "scope" (the first two path segments, e.g. `f/foo`
// or `u/alice`); deeper segments become nested folders. Folder keys are
// `folder:<fullPath>`. See ForkDiffDrawer's tree builder.

/**
 * The folder key that contains an entry (for ArrowLeft "go to parent"), or
 * `undefined` when the entry sits at the top with no parent folder.
 *
 * - A scope folder (≤2 segments) has no parent.
 * - A single-segment file leaf sits at the tree root with no scope folder.
 * - A file directly at its scope (`f/foo` or `f/foo/bar`) belongs to the scope
 *   folder = first two segments (`folder:f/foo`).
 * - Anything deeper belongs to its immediate folder.
 */
export function parentFolderKey(kind: 'folder' | 'file', path: string): string | undefined {
	const parts = path.split('/')
	if (kind === 'folder' && parts.length <= 2) return undefined
	if (kind === 'file') {
		if (parts.length < 2) return undefined
		if (parts.length <= 3) return `folder:${parts.slice(0, 2).join('/')}`
	}
	return `folder:${parts.slice(0, -1).join('/')}`
}
