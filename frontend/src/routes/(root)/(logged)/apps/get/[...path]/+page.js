export function load({ params }) {
	return {
		stuff: { title: `App ${params.path}` }
	}
}
