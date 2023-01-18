import type { AppComponent } from './types'

export function defaultCode(component: string, language: string): string | undefined {
	return DEFAULT_CODES[component]?.[language]
}

const DEFAULT_CODES: Partial<Record<AppComponent['type'], Record<'deno' | 'python3', string>>> = {
	tablecomponent: {
		deno:
			`export async function main() {
	return [
		{
			"id": 1,
			"name": "A cell with a long name",
			"age": 42
		},
		{
			"id": 2,
			"name": "A briefer cell",
			"age": 84
		}
	]
}`,
		python3:
			`def main():
	return [
		{
			"id": 1,
			"name": "A cell with a long name",
			"age": 42
		},
		{
			"id": 2,
			"name": "A briefer cell",
			"age": 84
		}
	]`,
	},
	textcomponent: {
		deno:
			`export async function main() {
	return "foo"
}`,
		python3:
			`def main():
	return "foo"`,
	},
	barchartcomponent: {
		deno:
			`export async function main() {
	return {
		"data": [
			25,
			50,
			25
		],
		"labels": [
			"Bar",
			"Charts",
			"<3"
		]
	}
}`,
		python3:
			`def main():
	return {
		"data": [
			25,
			50,
			25
		],
		"labels": [
			"Bar",
			"Charts",
			"<3"
		]
	}`,
	},
	displaycomponent: {
		deno:
			`export async function main() {
	return {
		"foo": 42
	}
}`,
		python3:
			`def main():
	return {
		"foo": 42
	}`,
	},
	htmlcomponent: {
		deno:
			`export async function main() {
	return \`<img
	src="https://images.unsplash.com/photo-1554629947-334ff61d85dc?ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&amp;ixlib=rb-1.2.1&amp;auto=format&amp;fit=crop&amp;w=1024&amp;h=1280&amp;q=80"
>
<h1 class="absolute top-4 left-2 text-white">
	Hello world
</h1>\`
}`,
		python3:
			`def main():
	return '''<img
	src="https://images.unsplash.com/photo-1554629947-334ff61d85dc?ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&amp;ixlib=rb-1.2.1&amp;auto=format&amp;fit=crop&amp;w=1024&amp;h=1280&amp;q=80"
>
<h1 class="absolute top-4 left-2 text-white">
	Hello world
</h1>'''`,
	},
	vegalitecomponent: {
		deno:
			`
			export async function main() {
				return {
					data: {
					values: [
						{ a: "A", b: 28 },
						{ a: "B", b: 55 },
						{ a: "C", b: 43 },
						{ a: "D", b: 91 },
					],
					},
					mark: "bar",
					encoding: {
						x: { field: "a", type: "ordinal" },
						y: { field: "b", type: "quantitative" },
					},
				};
			}			  
`,
		python3:
			`
def main():
	return {
		"data": {
		"values": [
			{ "a": "A", "b": 28 },
			{ "a": "B", "b": 55 },
			{ "a": "C", "b": 43 },
			{ "a": "D", "b": 91 },
		],
		},
			"mark": "bar",
		"encoding": {
			"x": { "field": "a", "type": "ordinal" },
			"y": { "field": "b", "type": "quantitative" },
		},
	}	
`
	},
	piechartcomponent: {
		deno:
			`export async function main() {
	return {
		"data": [
			25,
			50,
			25
		],
		"labels": [
			"Pie",
			"Charts",
			"<3"
		]
	}
}`,
		python3:
			`def main():
	return {
		"data": [
			25,
			50,
			25
		],
		"labels": [
			"Pie",
			"Charts",
			"<3"
		]
	}`,
	},
	scatterchartcomponent: {
		deno:
			`export async function main() {
	return [
		{
			"label": "foo",
			"data": [
				{ "x": 25, "y": 50 },
				{ "x": 23, "y": 23 },
				{ "x": 12, "y": 37 }
			],
			"backgroundColor": "rgb(255, 12, 137)"
		},
		{
			"label": "bar",
			"data": [
				{ "x": 32, "y": 32 },
				{ "x": 25, "y": 42 },
				{ "x": 3, "y": 27 }
			],
			"backgroundColor": "orange"
		}
	]
}`,
		python3:
			`def main():
	return [
		{
			"label": "foo",
			"data": [
				{ "x": 25, "y": 50 },
				{ "x": 23, "y": 23 },
				{ "x": 12, "y": 37 }
			],
			"backgroundColor": "rgb(255, 12, 137)"
		},
		{
			"label": "bar",
			"data": [
				{ "x": 32, "y": 32 },
				{ "x": 25, "y": 42 },
				{ "x": 3, "y": 27 }
			],
			"backgroundColor": "orange"
		}
	]`,
	},
	timeseriescomponent: {
		deno:
			`export async function main() {
	return [
		{
			"label": "foo",
			"data": [
				{
					"x": "2021-11-06 23:39:30",
					"y": 50
				},
				{
					"x": "2021-11-07 01:00:28",
					"y": 60
				},
				{
					"x": "2021-11-07 09:00:28",
					"y": 20
				}
			],
			"backgroundColor": "rgb(255, 12, 137)"
		},
		{
			"label": "bar",
			"data": [
				{
					"x": "2021-11-06 23:39:30",
					"y": 20
				},
				{
					"x": "2021-11-07 01:00:28",
					"y": 13
				},
				{
					"x": "2021-11-07 09:00:28",
					"y": 45
				}
			],
			"backgroundColor": "orange"
		}
	]
}`,
		python3:
			`def main():
	return [
		{
			"label": "foo",
			"data": [
				{
					"x": "2021-11-06 23:39:30",
					"y": 50
				},
				{
					"x": "2021-11-07 01:00:28",
					"y": 60
				},
				{
					"x": "2021-11-07 09:00:28",
					"y": 20
				}
			],
			"backgroundColor": "rgb(255, 12, 137)"
		},
		{
			"label": "bar",
			"data": [
				{
					"x": "2021-11-06 23:39:30",
					"y": 20
				},
				{
					"x": "2021-11-07 01:00:28",
					"y": 13
				},
				{
					"x": "2021-11-07 09:00:28",
					"y": 45
				}
			],
			"backgroundColor": "orange"
		}
	]`,
	},
} as const
