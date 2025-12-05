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

/**
 * Skip (mark as complete) all tutorials in a specific set of indexes
 */
export async function skipTutorialsByIndexes(tutorialIndexes: number[]) {
	const currentTodos = get(tutorialsToDo)
	const aft = currentTodos.filter((x) => !tutorialIndexes.includes(x))
	tutorialsToDo.set(aft)
	
	// Get current progress bits
	const currentResponse = await UserService.getTutorialProgress()
	let bits: number = currentResponse.progress ?? 0
	
	// Set bits for the specified indexes
	for (const index of tutorialIndexes) {
		const mask = 1 << index
		bits = bits | mask
	}
	
	// Only set skipped_all to true if ALL tutorials are now complete
	const allComplete = aft.length === 0
	await UserService.updateTutorialProgress({ 
		requestBody: { 
			progress: bits, 
			skipped_all: allComplete 
		} 
	})
}

/**
 * Reset (mark as incomplete) all tutorials in a specific set of indexes
 */
export async function resetTutorialsByIndexes(tutorialIndexes: number[]) {
	const currentTodos = get(tutorialsToDo)
	const aft = [...new Set([...currentTodos, ...tutorialIndexes])]
	tutorialsToDo.set(aft)
	skippedAll.set(false)
	
	// Get current progress bits
	const currentResponse = await UserService.getTutorialProgress()
	let bits: number = currentResponse.progress ?? 0
	
	// Clear bits for the specified indexes
	for (const index of tutorialIndexes) {
		const mask = 1 << index
		bits = bits & ~mask
	}
	
	await UserService.updateTutorialProgress({ 
		requestBody: { 
			progress: bits, 
			skipped_all: false 
		} 
	})
}

/**
 * Reset (mark as incomplete) a single tutorial by index
 */
export async function resetTutorialByIndex(tutorialIndex: number) {
	const currentTodos = get(tutorialsToDo)
	
	// Add the tutorial index back to todos if not already present
	if (!currentTodos.includes(tutorialIndex)) {
		const aft = [...currentTodos, tutorialIndex]
		tutorialsToDo.set(aft)
		skippedAll.set(false)
		
		// Get current progress bits
		const currentResponse = await UserService.getTutorialProgress()
		let bits: number = currentResponse.progress ?? 0
		
		// Clear bit for this tutorial index
		const mask = 1 << tutorialIndex
		bits = bits & ~mask
		
		await UserService.updateTutorialProgress({ 
			requestBody: { 
				progress: bits, 
				skipped_all: false 
			} 
		})
	}
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
