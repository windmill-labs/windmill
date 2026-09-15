export const zIndexes = {
	appEditorComponents: 1000,
	secondaryMenu: 1001,
	splitPanelHandle: 1001,
	colorInput: 1002,
	disposables: 1100, // Modals and Drawers
	aiChat: 1200,
	// Above every modal and drawer, AI chat's raised base included: it is opened from inside them
	// (the rename warning's content search) and takes no z-index from their stack.
	globalSearch: 1500,
	svelteSelectOptions: 5000,
	popover: 5001,
	contextMenu: 6000,
	draggingComponent: 10000,
	monacoEditor: 10000,
	monacoEditorSuggestions: 10001,
	tooltip: 20000
}
