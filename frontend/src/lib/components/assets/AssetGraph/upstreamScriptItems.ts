import type { Item } from '$lib/utils'
import type { ScriptLang } from '$lib/gen'
import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
import { Code2 } from 'lucide-svelte'

export type UpstreamScript = { path: string; summary?: string; language?: ScriptLang }

/** A menu of the scripts that produce an asset, to pick the one to open. */
export function upstreamScriptItems(
	scripts: UpstreamScript[],
	onPick: (path: string) => void
): Item[] {
	return scripts.map((s) => ({
		displayName: s.summary || s.path,
		description: s.summary ? s.path : undefined,
		...(s.language
			? { icon: LanguageIcon, iconProps: { lang: s.language, width: 14, height: 14 } }
			: { icon: Code2 }),
		action: () => onPick(s.path)
	}))
}

/** An upstream trigger the asset pane can open, and how. */
export type UpstreamTriggerAction = { label: string; detail?: string; onOpen: () => void }

export function upstreamTriggerItems(triggers: UpstreamTriggerAction[]): Item[] {
	return triggers.map((t) => ({
		displayName: t.label,
		description: t.detail,
		action: () => t.onOpen()
	}))
}
