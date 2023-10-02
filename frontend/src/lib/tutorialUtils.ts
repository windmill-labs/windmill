import { get } from 'svelte/store'
import { tutorialsToDo } from './stores'
import { UserService } from './gen'

const MAX_TUTORIAL_ID = 6

export async function updateProgress(id: number) {
	const bef = get(tutorialsToDo)
	const aft = bef.filter((x) => x != id)
	tutorialsToDo.set(aft)
	let bits = 0
	for (let i = 0; i <= MAX_TUTORIAL_ID; i++) {
		let mask = 1 << i
		if (!aft.includes(i)) {
			bits = bits | mask
		}
	}
	await UserService.updateTutorialProgress({ requestBody: { progress: bits } })
}

export async function skipAllTodos() {
	let bits = 0
	for (let i = 0; i <= MAX_TUTORIAL_ID; i++) {
		let mask = 1 << i
		bits = bits | mask
	}
	tutorialsToDo.set([])
	await UserService.updateTutorialProgress({ requestBody: { progress: bits } })
}

export async function resetAllTodos() {
	let todos: number[] = []
	for (let i = 0; i <= MAX_TUTORIAL_ID; i++) {
		todos.push(i)
	}
	tutorialsToDo.set(todos)

	await UserService.updateTutorialProgress({ requestBody: { progress: 0 } })
}

export async function syncTutorialsTodos() {
	const bits: number = (await UserService.getTutorialProgress()).progress!
	const todos: number[] = []
	for (let i = 0; i <= MAX_TUTORIAL_ID; i++) {
		let mask = 1 << i
		if ((bits & mask) == 0) {
			todos.push(i)
		}
	}
	tutorialsToDo.set(todos)
}
