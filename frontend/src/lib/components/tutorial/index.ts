import { browser } from '$app/environment'
import type Shepherd from 'shepherd.js'
import welcome from './welcome-steps'
import script from './script-steps'
import flow from './flow-steps'
import app from './app-steps'
import { writable } from 'svelte/store'

export { default as Tour } from './Tour.svelte'

export const TUTORIAL_NAMES = ['welcome', 'script', 'flow', 'app'] as const
export type TutorialName = (typeof TUTORIAL_NAMES)[number]

export const steps: Record<TutorialName, (tour: Shepherd.Tour) => object[] | Shepherd.Step[]> = {
	welcome,
	script,
	flow,
	app
}

const STORAGE_KEY = 'tutorials-to-show' as const
const store = writable<Readonly<TutorialName[]>>([])
export const tourStore = {
	subscribe: store.subscribe,
	initiateTours: () => {
		if (!browser) {
			return
		}
		window.localStorage.setItem(STORAGE_KEY, JSON.stringify(TUTORIAL_NAMES))
		store.set(TUTORIAL_NAMES)
	},
	isTourActive: (name: TutorialName) => {
		if (!browser) {
			return false
		}
		const toursToShow = JSON.parse(
			window.localStorage.getItem(STORAGE_KEY) || '[]'
		) as TutorialName[]
		return toursToShow.includes(name)
	},
	markTourAsDone: (name: TutorialName) => {
		if (!browser) {
			return
		}
		const toursToShow = (
			JSON.parse(window.localStorage.getItem(STORAGE_KEY) || '[]') as TutorialName[]
		).filter((t) => t !== name)
		window.localStorage.setItem(STORAGE_KEY, JSON.stringify(toursToShow))
		store.set(toursToShow)
	}
}
