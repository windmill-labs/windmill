import type { App } from './types'
import { gridColumns } from './gridUtils'
import { allItems } from './editor/appUtilsCore'

/**
 * Normalize an `App` in place to the current schema: default `hiddenInlineScripts`
 * type, migrate the legacy `doNotRecomputeOnInputChanged` flag, default a missing
 * `grid`, and default `fullHeight` on every grid item. Lives in its own light
 * module (no app-editor component imports) so non-editor callers, e.g. the
 * localStorage→DB draft migration, can reuse it without pulling the whole
 * `apps/utils` graph.
 */
export function migrateApp(app: App) {
	;(app?.hiddenInlineScripts ?? []).forEach((x) => {
		if (x.type == undefined) {
			//@ts-ignore
			x.type = 'inline'
		}
		//TODO: remove after migration is done
		if (x.doNotRecomputeOnInputChanged != undefined) {
			x.recomputeOnInputChanged = !x.doNotRecomputeOnInputChanged
			x.doNotRecomputeOnInputChanged = undefined
		}
	})

	// A stored app value can have no `grid` at all: a persisted draft row is the
	// confirmed case, and the editor renders a draft in place of the deployed
	// value. The grid components dereference it unguarded, so normalize it here,
	// the one hook every load path runs through.
	if (!Array.isArray(app.grid)) {
		app.grid = []
	}

	allItems(app.grid, app.subgrids).forEach((x) => {
		gridColumns.forEach((column: number) => {
			if (x?.[column]?.fullHeight === undefined) {
				x[column].fullHeight = false
			}
		})
	})
}
