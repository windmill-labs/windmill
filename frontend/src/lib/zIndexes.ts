export const zIndexes = {
	appEditorComponents: 1000,
	secondaryMenu: 1001,
	splitPanelHandle: 1001,
	colorInput: 1002,
	disposables: 1100, // Modals and Drawers
	aiChat: 1200,
	// Above the modal and drawer bases (`disposables`, or `aiChat + 1` while the chat is open) and the
	// chat panel: it is opened from inside modals (the rename warning's content search) and takes no
	// z-index from their stack. A disposable raised past it with `minZIndex` still covers it.
	globalSearch: 1500,
	svelteSelectOptions: 5000,
	popover: 5001,
	contextMenu: 6000,
	draggingComponent: 10000,
	monacoEditor: 10000,
	monacoEditorSuggestions: 10001,
	tooltip: 20000
}
