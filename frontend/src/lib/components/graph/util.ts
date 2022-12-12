export const NODE = {
	width: 300,
	height: 35,
	gap: {
		horizontal: 40,
		vertical: 50
	}
}

export function* createIdGenerator(): Generator<number, number, unknown> {
	let id = 0
	while (true) {
		yield id++
	}
}

