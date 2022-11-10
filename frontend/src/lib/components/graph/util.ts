export const NODE = {
	width: 200,
	height: 40,
	gap: {
		horizontal: 20,
		vertical: 50
	}
}

export function* createIdGenerator(): Generator<number, number, unknown> {
	let id = 0
	while(true) {
		yield id++
	}
}