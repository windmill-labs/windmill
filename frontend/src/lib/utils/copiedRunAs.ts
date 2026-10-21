/**
 * A script or flow seeded from another one (a fork, a template, an import) is a new item of
 * whoever creates it. The backend keeps any identity a deploy names for an admin or deployer,
 * so the source's run-as identity would carry over to the copy: it is replaced by the
 * creator's own, keeping only *that* the source ran on behalf of someone.
 */
export function withCreatorRunAs<T extends { on_behalf_of_email?: string; on_behalf_of?: string }>(
	seed: T,
	creatorEmail: string | undefined
): T {
	if (!seed.on_behalf_of_email && !seed.on_behalf_of) return seed
	return { ...seed, on_behalf_of_email: creatorEmail, on_behalf_of: undefined }
}
