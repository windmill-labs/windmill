import { BROWSER } from 'esm-env'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'

// Per browser, like the composer's own pin: how a reader likes their Home laid out.
const HIDDEN_SETTING = 'home-quick-access-hidden'
const ROWS_SETTING = 'home-quick-access-rows'
const NO_FAVORITES_SETTING = 'home-quick-access-no-favorites'
const NO_RECENT_SETTING = 'home-quick-access-no-recent'

export const QUICK_ACCESS_ROWS = [1, 2, 3, 4] as const
export type QuickAccessRows = (typeof QUICK_ACCESS_ROWS)[number]
const DEFAULT_ROWS: QuickAccessRows = 2

// Blocked storage (some private windows) throws on write; the setting then lasts the visit.
function save(name: string, value: string | undefined) {
	try {
		storeLocalSetting(name, value)
	} catch {}
}

function readRows(): QuickAccessRows {
	const raw = getLocalSetting(ROWS_SETTING)
	return QUICK_ACCESS_ROWS.find((r) => String(r) === raw) ?? DEFAULT_ROWS
}

class HomeQuickAccessSettings {
	hidden = $state(BROWSER && getLocalSetting(HIDDEN_SETTING) === 'true')
	rows = $state<QuickAccessRows>(BROWSER ? readRows() : DEFAULT_ROWS)
	showFavorites = $state(!(BROWSER && getLocalSetting(NO_FAVORITES_SETTING) === 'true'))
	showRecent = $state(!(BROWSER && getLocalSetting(NO_RECENT_SETTING) === 'true'))
	/** Bumped on each hide, so the header menu that holds the way back can flash. */
	hideCount = $state(0)

	setHidden(next: boolean) {
		this.hidden = next
		if (next) this.hideCount++
		save(HIDDEN_SETTING, next ? 'true' : undefined)
	}

	setShowFavorites(next: boolean) {
		this.showFavorites = next
		save(NO_FAVORITES_SETTING, next ? undefined : 'true')
	}

	setShowRecent(next: boolean) {
		this.showRecent = next
		save(NO_RECENT_SETTING, next ? undefined : 'true')
	}

	setRows(next: QuickAccessRows) {
		this.rows = next
		save(ROWS_SETTING, next === DEFAULT_ROWS ? undefined : String(next))
	}
}

export const homeQuickAccessSettings = new HomeQuickAccessSettings()
