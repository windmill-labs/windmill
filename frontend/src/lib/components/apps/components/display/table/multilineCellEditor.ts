import type { ColDef, ICellEditorComp, ICellEditorParams } from 'ag-grid-community'

/** Kept in step with the `line-height` the stylesheet gives the textarea. */
const LINE_HEIGHT = 20

/**
 * A text cell editor that starts the height of the cell and grows as lines are added, for columns
 * holding prose rather than a value. Enter commits, Shift+Enter adds a line, Escape cancels.
 *
 * AgGrid's own two editors are the ends of this: `agTextCellEditor` is an `<input>` and cannot hold
 * a newline at all, and `agLargeTextCellEditor` opens a fixed box of ten rows under the cell for
 * every edit, whatever is in it. A cell that looks like the row it is in until there is a reason
 * for it not to is what a grid of text wants.
 *
 * Rendered as a popup positioned over the cell: an in-cell editor is clipped to the row height, so
 * growing is only visible if the editor is allowed to paint outside it.
 */
export class MultilineCellEditor implements ICellEditorComp {
	private eGui!: HTMLDivElement
	private textarea!: HTMLTextAreaElement
	private params!: ICellEditorParams

	init(params: ICellEditorParams) {
		this.params = params
		this.eGui = document.createElement('div')
		this.eGui.className = 'wm-multiline-cell-editor'

		this.textarea = document.createElement('textarea')
		this.textarea.rows = 1
		// The starting text: a keystroke that opened the edit replaces the value, the way it does in
		// every other cell; F2 and double-click keep it to be edited.
		this.textarea.value = params.eventKey?.length === 1 ? params.eventKey : (params.value ?? '')
		this.textarea.style.width = `${params.column.getActualWidth() - 2}px`
		// Padded, rather than given a line-height of a whole row, so that one line fills the cell it
		// replaces and a second one costs a line instead of another row. From the row rather than
		// from `--ag-row-height`, which is the theme's figure and not necessarily this grid's.
		const rowHeight = params.node.rowHeight ?? 28
		const padding = Math.max(0, (rowHeight - LINE_HEIGHT - 2) / 2)
		this.textarea.style.paddingTop = `${padding}px`
		this.textarea.style.paddingBottom = `${padding}px`

		this.textarea.addEventListener('input', () => this.resize())
		this.textarea.addEventListener('keydown', (e) => {
			if (e.key === 'Escape') {
				// Kept from whatever is around the grid: a grid in a drawer or a dialog is under a
				// surface that closes on Escape, and leaving an edit is not asking to leave that.
				e.preventDefault()
				e.stopPropagation()
				this.params.api.stopEditing(true)
				return
			}
			if (e.key !== 'Enter') return
			// Both branches keep the key from the grid, which ends the edit on Enter whether or not
			// Shift is held. Shift+Enter then falls through to the textarea's own default, which is
			// the newline; plain Enter is commit, so this ends the edit itself.
			e.stopPropagation()
			if (!e.shiftKey) {
				e.preventDefault()
				this.params.stopEditing()
			}
		})
		this.eGui.appendChild(this.textarea)
	}

	private resize() {
		this.textarea.style.height = 'auto'
		this.textarea.style.height = `${this.textarea.scrollHeight}px`
	}

	getGui() {
		return this.eGui
	}

	afterGuiAttached() {
		this.resize()
		this.textarea.focus()
		// At the end rather than selected: an edit reached by double-click or F2 is one you meant to
		// continue, and a selection there is a keystroke away from erasing the cell.
		const end = this.textarea.value.length
		this.textarea.setSelectionRange(end, end)
	}

	getValue() {
		return this.textarea.value
	}

	isPopup() {
		return true
	}

	getPopupPosition(): 'over' | 'under' {
		return 'over'
	}
}

/**
 * What a column of prose needs, ready to spread into a colDef. `suppressKeyboardEvent` as well as
 * the editor itself: the grid ends an edit on Enter from its own handler, which a popup editor's
 * DOM does not sit under, so the editor cannot keep Shift+Enter for itself on its own.
 */
export const multilineCellColDef: Pick<ColDef, 'cellEditor' | 'suppressKeyboardEvent'> = {
	cellEditor: MultilineCellEditor,
	suppressKeyboardEvent: (p) =>
		p.editing && (p.event as KeyboardEvent).key === 'Enter' && (p.event as KeyboardEvent).shiftKey
}
