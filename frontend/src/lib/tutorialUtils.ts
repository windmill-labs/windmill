import { get } from 'svelte/store'
import { tutorialsToDo, skippedAll } from './stores'
import { UserService } from './gen'

const MAX_TUTORIAL_ID = 7

/**
 * Helper function to calculate tutorial progress for a given set of tutorial indexes.
 * Returns total count. For completed count, use in component with reactive store access.
 */
export function getTutorialProgressTotal(tutorialIndexes: Record<string, number>): number {
	return Object.values(tutorialIndexes).length
}

/**
 * Helper function to calculate completed tutorials count.
 * Must be called with current tutorialsToDo array.
 */
export function getTutorialProgressCompleted(
	tutorialIndexes: Record<string, number>,
	tutorialsToDoArray: number[]
): number {
	return Object.values(tutorialIndexes).filter((index) => !tutorialsToDoArray.includes(index))
		.length
}

export async function updateProgress(id: number) {
	const bef = get(tutorialsToDo)
	const aft = bef.filter((x) => x != id)
	tutorialsToDo.set(aft)
	skippedAll.set(false) // Mark as not skipped when completing a tutorial
	let bits = 0
	for (let i = 0; i <= MAX_TUTORIAL_ID; i++) {
		let mask = 1 << i
		if (!aft.includes(i)) {
			bits = bits | mask
		}
	}
	await UserService.updateTutorialProgress({ requestBody: { progress: bits, skipped_all: false } })
}

export async function skipAllTodos() {
	let bits = 0
	for (let i = 0; i <= MAX_TUTORIAL_ID; i++) {
		let mask = 1 << i
		bits = bits | mask
	}
	tutorialsToDo.set([])
	skippedAll.set(true)
	await UserService.updateTutorialProgress({ requestBody: { progress: bits, skipped_all: true } })
}

export async function resetAllTodos() {
	let todos: number[] = []
	for (let i = 0; i <= MAX_TUTORIAL_ID; i++) {
		todos.push(i)
	}
	tutorialsToDo.set(todos)
	skippedAll.set(false)

	await UserService.updateTutorialProgress({ requestBody: { progress: 0, skipped_all: false } })
}

export async function syncTutorialsTodos() {
	const response = await UserService.getTutorialProgress()
	const bits: number = response.progress!
	const skipped: boolean = response.skipped_all ?? false
	const todos: number[] = []
	for (let i = 0; i <= MAX_TUTORIAL_ID; i++) {
		let mask = 1 << i
		if ((bits & mask) == 0) {
			todos.push(i)
		}
	}
	tutorialsToDo.set(todos)
	skippedAll.set(skipped)
}

export function tutorialInProgress() {
	const svg = document.getElementsByClassName('driver-overlay driver-overlay-animated')

	return svg.length > 0
}

/**
 * Check if tutorials should be hidden from the main menu.
 * Returns true if all tutorials are completed OR user skipped all.
 */
export function shouldHideTutorialsFromMainMenu(): boolean {
	const todos = get(tutorialsToDo)
	const skipped = get(skippedAll)
	// Hide if all tutorials are completed OR user skipped all
	return todos.length === 0 || skipped
}
