// What makes an artifact the session's plan, in one place. Import-free on purpose, like
// artifactLimits: the eval harness asks the same question from bun, where artifactsDB's `idb`
// and `$lib` imports cannot be resolved.

/** A session holds one plan, so its id is the session's — the primary key is the constraint,
 * and no two writers can mint a second row for the same session. */
export function planArtifactId(sessionId: string): string {
	return `plan:${sessionId}`
}

/**
 * Either mark is enough, so a row carrying only one of them still reads as the plan — the
 * fail-closed answer for callers that must not touch it by accident.
 *
 * Structurally typed rather than taking a PersistedArtifact, which would cost the import this
 * module exists to avoid.
 */
export function isPlanArtifact(a: { id: string; role?: 'plan' }, sessionId: string): boolean {
	return a.role === 'plan' || a.id === planArtifactId(sessionId)
}
