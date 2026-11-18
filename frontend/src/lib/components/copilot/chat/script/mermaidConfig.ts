import type { MermaidConfig } from 'mermaid'

/** The settings every diagram is drawn with, in the app and in an artifact's HTML download. */
export function mermaidConfig(dark: boolean): MermaidConfig {
	return {
		startOnLoad: false,
		theme: dark ? 'dark' : 'default',
		securityLevel: 'strict',
		// Mermaid writes this stack into a <style> block inside the SVG, so the app's font
		// never reaches a diagram: the bundled vector emoji font has to be named here or
		// emoji fall back to the platform bitmap one, which ignores the zoom transform
		// (app.css). Inter sits last before it because the emoji font also covers digits,
		// # and *; on a box without the MS core fonts those would otherwise render as its
		// keycap glyphs. Keeping Inter behind them leaves diagram typography unchanged.
		fontFamily: '"trebuchet ms", verdana, arial, Inter, "Noto Color Emoji", sans-serif',
		// Throw on parse errors instead of injecting an orphan error diagram into the DOM.
		suppressErrorRendering: true
	}
}
