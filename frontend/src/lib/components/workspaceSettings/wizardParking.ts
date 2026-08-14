/**
 * Where popups are blocked the Supabase leg falls back to a full-page redirect, which
 * unmounts the wizard. What the user had chosen is parked here and picked back up by the
 * settings page when Supabase sends them home.
 *
 * Kept out of the wizard component so the OAuth callback route can ask whether anything is
 * parked without pulling the whole wizard into that page's bundle.
 */

const RESUME_KEY = 'datatable_wizard_resume'

export type WizardResume = {
	name: string
	region: string
	projectName: string
	/**
	 * What the interrupted run had already created. Without these the resumed run meets its
	 * own secret variable and resource as somebody else's and refuses to write over them,
	 * which strands the Supabase project it just paid for. No secret is parked -- these are
	 * paths, and the password they name is already in the workspace.
	 */
	resourcePath?: string
	claimedPath?: string
	createdProjectName?: string
	createdProjectPath?: string
}

/** True while a wizard run is waiting on the Supabase redirect to come back. */
export function hasParkedWizard(): boolean {
	return sessionStorage.getItem(RESUME_KEY) != null
}

export function parkWizard(state: WizardResume) {
	sessionStorage.setItem(RESUME_KEY, JSON.stringify(state))
}

export function takeParkedWizard(): WizardResume | undefined {
	const raw = sessionStorage.getItem(RESUME_KEY)
	sessionStorage.removeItem(RESUME_KEY)
	if (!raw) return undefined
	try {
		return JSON.parse(raw)
	} catch {
		return undefined
	}
}
