/** Marker prefix for draft secret values the backend encrypted at rest
 * with the workspace key (mirrors `ENCRYPTED_DRAFT_PREFIX` in
 * `backend/windmill-common/src/user_drafts.rs`). The plaintext cannot be
 * recovered client-side — deploying sends the marker as-is and the
 * deploy endpoints decrypt it server-side. */
export const ENCRYPTED_DRAFT_PREFIX = '$encrypted:'

export function isEncryptedDraftValue(v: unknown): boolean {
	return typeof v === 'string' && v.startsWith(ENCRYPTED_DRAFT_PREFIX)
}
