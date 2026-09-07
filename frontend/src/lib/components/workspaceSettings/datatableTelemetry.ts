import { logFeatureUsage } from '$lib/utils/featureUsage'

// Anonymous counters for the data table surfaces the backend cannot see: which substrate the
// add-wizard is pointed at and how far a run gets, and what the DDL guard talks people into.
// Same rules as every other `logFeatureUsage` caller: aggregated counts only, and the keys
// below are the whole vocabulary — no data table name, connection string, resource path or SQL
// ever reaches here.

/** The substrate a wizard run is pointed at. Mirrors the wizard's own `Provider`. */
export type DatatableWizardProvider = 'supabase' | 'instance' | 'resource'

export type DatatableWizardEvent =
	/** The wizard was opened, including a run resumed from the Supabase redirect. */
	| { step: 'opened' }
	/** A substrate was picked. Re-picking a different one counts again, by design: the
	 *  abandoned branch is the interesting half of a funnel. */
	| { step: 'picked'; provider: DatatableWizardProvider }
	/** A run finished, with the verdict the checklist reported. */
	| { step: 'done' | 'failed'; provider: DatatableWizardProvider }

export function logDatatableWizard(event: DatatableWizardEvent): void {
	const key = event.step === 'opened' ? 'opened' : `${event.step}_${event.provider}`
	logFeatureUsage('datatable', 'wizard', { key })
}

export type DdlGuardChoice =
	/** The DDL was run ad-hoc, against the guard's advice. */
	| 'run_anyway'
	/** The DDL became a migration definition. */
	| 'migrated'
	/** The statement was abandoned, so nothing ran. */
	| 'cancelled'

/**
 * Counted once per prompt that reaches a terminal choice. Picking "create a migration" and then
 * dismissing the modal loops back to the prompt instead, and is deliberately not counted: it is
 * the same statement still undecided, not a fourth outcome.
 */
export function logDdlGuardChoice(choice: DdlGuardChoice): void {
	logFeatureUsage('datatable', 'ddl_guard', { key: choice })
}
