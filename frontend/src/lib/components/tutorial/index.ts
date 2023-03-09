import welcome from './welcome-steps'
import type Shepherd from 'shepherd.js'

export { default as Tour } from './Tour.svelte'

export type TutorialName = 'welcome' | 'script' | 'flow' | 'app'
export const steps: Record<TutorialName, (tour: Shepherd.Tour) => object[] | Shepherd.Step[]> = {
	welcome,
	script: (tour: Shepherd.Tour) => [],
	flow: (tour: Shepherd.Tour) => [],
	app: (tour: Shepherd.Tour) => []
}
