import { createPathsPreference } from '../enabledPathsPreference'

/** Which `ai_skill` resources the chat may follow, per workspace and per account.
 * Every skill readable in the workspace is on unless someone turned it off: a skill
 * is instructions the workspace wrote for the assistant to use, so what carrying one
 * costs is context, not access. */
const preference = createPathsPreference('wm_skills_enabled', true)

export const isSkillEnabled = preference.isEnabled
export const setSkillEnabled = preference.setEnabled
export const forgetSkill = preference.forget
