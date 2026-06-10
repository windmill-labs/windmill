/** Opaque placeholder the server sends in place of a draft secret's
 * value (mirrors `DRAFT_SECRET_SENTINEL` in
 * `backend/windmill-common/src/user_drafts.rs`). The real secret —
 * encrypted at rest with the workspace key — NEVER leaves the server:
 * `get_variable` swaps the ciphertext for this sentinel. Deploying sends
 * the sentinel back and the server rehydrates the plaintext from the
 * caller's own draft row. A field holding this value is "secret set,
 * hidden, unchanged" — masked in the UI, not editable in place. */
export const DRAFT_SECRET_SENTINEL = '$draft_secret'

export function isEncryptedDraftValue(v: unknown): boolean {
	return v === DRAFT_SECRET_SENTINEL
}
