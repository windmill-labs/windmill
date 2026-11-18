import type { WorkspaceItem } from '$lib/components/workspacePicker'
import { barePath, isPreviewListPath } from './previewPaths'
import { previewLocationLabel } from './previewRouter'
import type { SessionPreviewTabs } from './sessionPreviewTabs.svelte'

/**
 * A preview frame hands some navigations up rather than making them itself (the logged
 * layout's beforeNavigate); applied here to the panel showing that frame, `owner`.
 */
export function onPreviewFrameMessage(e: MessageEvent, owner: SessionPreviewTabs | undefined) {
	if (e.origin !== window.location.origin) return
	const d = e.data
	if (!d) return
	// A preview frame navigating to an editor route: re-point the active tab to the live
	// in-process editor instead of booting a second one in the frame.
	if (d.type === 'wm.session.openEditor') {
		if (d.kind !== 'script' && d.kind !== 'flow' && d.kind !== 'raw_app') return
		if (typeof d.path !== 'string') return
		const item: WorkspaceItem =
			d.kind === 'raw_app'
				? { kind: 'app', raw_app: true, path: d.path, summary: '' }
				: { kind: d.kind, path: d.path, summary: '' }
		owner?.navigate({ type: 'item', item })
		return
	}
	// A preview frame navigating to a list page: re-point the active tab to the list mounted in
	// process, whose rows open as tabs of their own.
	if (d.type === 'wm.session.openList') {
		if (typeof d.href !== 'string' || !isPreviewListPath(barePath(d.href))) return
		owner?.navigate({ type: 'page', href: d.href, label: previewLocationLabel(d.href) })
		return
	}
	// A job clicked inside a preview tab: open the run detail in a NEW tab so the originating
	// page (e.g. Runs) stays put. open() focuses an existing tab for the same run rather than
	// duplicating it.
	if (d.type === 'wm.session.openRun') {
		if (typeof d.href !== 'string') return
		owner?.open({
			type: 'page',
			href: d.href,
			label: typeof d.label === 'string' ? d.label : 'Run'
		})
	}
}
