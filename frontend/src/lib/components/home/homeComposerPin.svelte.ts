import { BROWSER } from 'esm-env'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'

// Whether this reader put the composer away from the home page, so it lives per browser
// rather than per workspace or account. It has nothing to do with the composer's one-line
// state despite the key's name; renaming the key would reset every reader's choice.
const REMOVED_SETTING = 'home-ai-composer-collapsed'

export const homeComposerPin = $state({
	removed: BROWSER && getLocalSetting(REMOVED_SETTING) === 'true'
})

export function setHomeComposerRemoved(removed: boolean): void {
	homeComposerPin.removed = removed
	storeLocalSetting(REMOVED_SETTING, removed ? 'true' : undefined)
}

/** Starting a session asks for the composer: it is the only place one is typed. */
export function pinHomeComposer(): void {
	if (homeComposerPin.removed) setHomeComposerRemoved(false)
}
