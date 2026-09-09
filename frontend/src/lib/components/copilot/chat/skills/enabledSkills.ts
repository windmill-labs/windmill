import { createEnabledPathsPreference } from '../enabledPathsPreference'

/** Which `ai_skill` resources the chat may follow, per workspace and per account.
 * Every enabled skill spends context on every turn, so selecting one is a personal
 * choice rather than a consequence of being able to read it. */
const preference = createEnabledPathsPreference('wm_skills_enabled')

export const enabledSkillPaths = preference.enabledPaths
export const isSkillEnabled = preference.isEnabled
export const setSkillEnabled = preference.setEnabled
