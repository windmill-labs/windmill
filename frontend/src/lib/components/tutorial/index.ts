import type Shepherd from 'shepherd.js'
import welcome from './welcome-steps'
import script from './script-steps'
import flow from './flow-steps'
import app from './app-steps'

export { default as Tour } from './Tour.svelte'

export type TutorialName = 'welcome' | 'script' | 'flow' | 'app'
export const steps: Record<TutorialName, (tour: Shepherd.Tour) => object[] | Shepherd.Step[]> = {
	welcome,
	script,
	flow,
	app
}
