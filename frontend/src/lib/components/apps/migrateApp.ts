import type { App } from './types'
import { gridColumns } from './gridUtils'
import { allItems } from './editor/appUtilsCore'

/**
 * Normalize an `App` in place to the current schema: default `hiddenInlineScripts`
 * type, migrate the legacy `doNotRecomputeOnInputChanged` flag, and default
 * `fullHeight` on every grid item. Lives in its own light module (no app-editor
 * component imports) so non-editor callers — e.g. the localStorage→DB draft
 * migration — can reuse it without pulling the whole `apps/utils` graph.
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

	allItems(app.grid, app.subgrids).forEach((x) => {
		gridColumns.forEach((column: number) => {
			if (x?.[column]?.fullHeight === undefined) {
				x[column].fullHeight = false
			}
		})
	})
}
