import { describe, it, expect } from 'vitest'
import { resolvePanelMode, MODAL_PANEL_BREAKPOINT } from './panelPlacement'

const auto = { enabled: true, preference: 'auto' as const }

describe('resolvePanelMode', () => {
	it('detaches on auto once the editor is narrower than the breakpoint', () => {
		expect(resolvePanelMode({ ...auto, width: MODAL_PANEL_BREAKPOINT - 1 })).toBe('modal')
		expect(resolvePanelMode({ ...auto, width: MODAL_PANEL_BREAKPOINT })).toBe('docked')
	})

	it('stays docked at zero width, which means unlaid-out rather than narrow', () => {
		expect(resolvePanelMode({ ...auto, width: 0 })).toBe('docked')
	})

	it('holds a pinned preference against the width that contradicts it', () => {
		const wide = MODAL_PANEL_BREAKPOINT + 400
		expect(resolvePanelMode({ enabled: true, preference: 'modal', width: wide })).toBe('modal')
		expect(resolvePanelMode({ enabled: true, preference: 'docked', width: 400 })).toBe('docked')
	})

	it('keeps whitelabel embeds docked whatever was picked', () => {
		expect(resolvePanelMode({ enabled: false, preference: 'modal', width: 400 })).toBe('docked')
	})
})
