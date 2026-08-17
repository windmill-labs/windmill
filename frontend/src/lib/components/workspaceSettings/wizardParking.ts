/**
 * Where popups are blocked the Supabase leg falls back to a full-page redirect, which
 * unmounts the wizard. What the user had chosen is parked here and picked back up by the
 * settings page when Supabase sends them home.
 *
 * Kept out of the wizard component so the OAuth callback route can ask whether anything is
 * parked without pulling the whole wizard into that page's bundle.
 */

import type { SupabaseConnectionMode, SupabaseOrg, SupabaseProject } from './supabaseProvisioning'
import type { Claim } from './setupClaims'
import type { CreatedProject } from './addDataTableModel'

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
	/** Everything the run holds, serialised whole so a newly added kind cannot be left behind. */
	claims?: Claim[]
	/** Every project created before the redirect, each still guarding its password's path. */
	createdProjects?: CreatedProject[]
	/**
	 * Which side of the step-2 toggle the run was on, and where it was pointed. A run that
	 * died mid-create otherwise comes back on `existing`, is asked for the password it
	 * generated and never showed anyone, and looks for its project in whichever organization
	 * happens to be first.
	 */
	mode?: 'existing' | 'create'
	org?: SupabaseOrg
	/** The project that was picked. Without it a resume selects the first in the list, which is
	 *  a different database from the one whose password the user had already typed. */
	project?: SupabaseProject
	connectionMode?: SupabaseConnectionMode
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
