import { createPathsPreference } from '../enabledPathsPreference'

// No agent tool or prompt builder reads this preference: it only controls the
// context-summary presentation and must not be interpreted as an access boundary.
const preference = createPathsPreference('wm_ai_editable_folders', true)

export const isEditableFolderSelected = preference.isEnabled
export const setEditableFolderSelected = preference.setEnabled
