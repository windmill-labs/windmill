import type { AppComponent } from '.'

export function defaultCode(component: string, language: string): string | undefined {
	return DEFAULT_CODES[component]?.[language]
}

export const DEFAULT_CODES: Partial<
	Record<
		AppComponent['type'],
		Partial<Record<'deno' | 'python3' | 'go' | 'bash' | 'pgsql' | 'mysql', string>>
	>
> = {
	tablecomponent: {
		deno: `export async function main() {
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
		python3: `def main():
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
		pgsql: `import { pgSql } from "npm:windmill-client@${__pkg__.version}";

type Postgresql = object

export async function main(db: Postgresql) {
    const query = await pgSql(db)\`SELECT * FROM demo;\`;
    return query.rows;
}`
	},
	aggridcomponent: {
		deno: `export async function main() {
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
		python3: `def main():
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
		pgsql: `import { pgSql } from "npm:windmill-client@${__pkg__.version}";

type Postgresql = object

export async function main(db: Postgresql) {
    const query = await pgSql(db)\`SELECT * FROM demo;\`;
    return query.rows;
}`
	},
	steppercomponent: {
		deno: `export async function main(stepIndex: number) {
        // if (stepIndex == 0) {
        //     if (page0Invalid) throw Error("first step invalid")
        // } else if ...
}`,
		python3: `def main(stepIndex: int):
# if stepIndex == 0:
#     if page0Invalid:
#         raise Exception("first step invalid")
# elif ...
`
	},
	textcomponent: {
		deno: `export async function main() {
    return "foo"
}`,
		python3: `def main():
    return "foo"`
	},
	barchartcomponent: {
		deno: `export async function main() {
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
		python3: `def main():
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
		pgsql: `import { pgSql } from "npm:windmill-client@${__pkg__.version}";

type Postgresql = object

export async function main(db: Postgresql) {
    const query = await pgSql(db)\`SELECT * FROM demo;\`;
    return {
        data: query.rows.map((row) => row['0']),
        labels: query.rows.map((row) => row['1']?.slice(0, 6))
    };
}`
	},
	displaycomponent: {
		deno: `export async function main(x = 42) {
    return {
        foo: x
    }
}`,
		python3: `def main():
    return {
        "foo": 42
    }`
	},
	htmlcomponent: {
		deno: `export async function main() {
    return \`<img
    src="https://images.unsplash.com/photo-1554629947-334ff61d85dc?ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&amp;ixlib=rb-1.2.1&amp;auto=format&amp;fit=crop&amp;w=1024&amp;h=1280&amp;q=80"
    >
    <h1 class="absolute top-4 left-2 text-white">
        Hello world
    </h1>\`
}`,
		python3: `def main():
    return '''<img
    src="https://images.unsplash.com/photo-1554629947-334ff61d85dc?ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&amp;ixlib=rb-1.2.1&amp;auto=format&amp;fit=crop&amp;w=1024&amp;h=1280&amp;q=80"
    >
    <h1 class="absolute top-4 left-2 text-white">
    Hello world
    </h1>'''`
	},
	vegalitecomponent: {
		deno: `export async function main() {
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
        }
    }
}`,
		python3: `def main():
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
    }`,
		pgsql: `import { pgSql } from "npm:windmill-client@${__pkg__.version}";

type Postgresql = object

export async function main(Postgresqlstgresql) {
    const query = await pgSql(db)\`SELECT * FROM demo;\`;
    return {
        data: {
            values: query.rows.map((row) => {
                row['1'] = row['1']?.slice(0, 6);
                return row;
            })
        },
        mark: "bar",
        encoding: {
            x: { field: "1", type: "ordinal" },
            y: { field: "0", type: "quantitative" },
        }
    };
}`
	},
	plotlycomponent: {
		deno: `export async function main() {
    return {
        type: 'bar',
        x: [1, 2, 3, 4],
        y: [5, 10, 2, 8],
        marker: {
            color: '#C8A2C8',
            line: {
                width: 2.5                  
            }
        }
    };
}`,
		python3: `def main():
    return {
        "type": "bar",
        "x": [1, 2, 3, 4],
        "y": [5, 10, 2, 8],
        "marker": {
            "color": "#C8A2C8",
            "line": {
                "width": 2.5                  
            }
        }
    }`,
		pgsql: `import { pgSql } from "npm:windmill-client@${__pkg__.version}";

type Postgresql = object

export async function main(Postgresqlstgresql) {
    const query = await pgSql(db)\`SELECT * FROM demo;\`;
    return {
        type: 'bar',
        x: query.rows.map((row) => row['1']?.slice(0, 6)),
        y: query.rows.map((row) => row['0']),
        marker: {
            color: '#C8A2C8',
            line: {
                width: 2.5                  
            }
        }
    };
}`
	},
	piechartcomponent: {
		deno: `export async function main() {
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
		python3: `def main():
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
		pgsql: `import { pgSql } from "npm:windmill-client@${__pkg__.version}";

type Postgresql = object

export async function main(db: Postgresql) {
    const query = await pgSql(db)\`SELECT * FROM demo;\`;
    return {
        data: query.rows.map((row) => row['0']),
        labels: query.rows.map((row) => row['1']?.slice(0, 6))
    };
}`
	},
	scatterchartcomponent: {
		deno: `export async function main() {
    return [
        {
            label: "foo",
            data: [
                { x: 25, y: 50 },
                { x: 23, y: 23 },
                { x: 12, y: 37 }
            ],
            backgroundColor: "rgb(255, 12, 137)"
        },
        {
            label: "bar",
            data: [
                { x: 32, y: 32 },
                { x: 25, y: 42 },
                { x: 3, y: 27 }
            ],
            backgroundColor: "orange"
        }
    ];
}`,
		python3: `def main():
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
		pgsql: `import { pgSql } from "npm:windmill-client@${__pkg__.version}";

type Postgresql = object

export async function main(db: Postgresql) {
    try {
        const query = await pgSql(db)\`SELECT * FROM demo;\`;
    const rows = query.rows.map((row, i) => ({
      x: row['0'],
      y: query.rows[query.rows.length - (i + 1)]['0']
    }))
        return [
      {
                label: "foo",
                data: rows,
                backgroundColor: "rgb(255, 12, 137)"
            },
            {
                label: "bar",
                data: rows.map(({x, y}) => ({
          x: x * 2,
          y: y * 1.5
        })),
                backgroundColor: "orange"
            }
    ];
    } catch (e) {
        return [];
    }
}`
	},
	timeseriescomponent: {
		deno: `export async function main() {
    return [
        {
            label: "foo",
            data: [
                {
                    x: "2021-11-06 23:39:30",
                    y: 50
                },
                {
                    x: "2021-11-07 01:00:28",
                    y: 60
                },
                {
                    x: "2021-11-07 09:00:28",
                    y: 20
                }
            ],
            backgroundColor: "rgb(255, 12, 137)"
        },
        {
            label: "bar",
            data: [
                {
                    x: "2021-11-06 23:39:30",
                    y: 20
                },
                {
                    x: "2021-11-07 01:00:28",
                    y: 13
                },
                {
                    x: "2021-11-07 09:00:28",
                    y: 45
                }
            ],
            backgroundColor: "orange"
        }
    ]
}`,
		python3: `def main():
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
    ]`
	},
	iconcomponent: {
		deno: `export async function main() {
    return "Smile";
}`,
		python3: `def main():
    return "Smile"`
	},
	schemaformcomponent: {
		deno: `export async function main() {
return {
        properties: {
            first_name: {
                type: 'string',
                description: 'your name',
                default: 'default'
            }
        },
        required: []
    }
}`,
		python3: `def main():
    return {
        "properties": {
            "first_name": {
                "type": "string",
                "description": "your name",
                "default": "default"
            }
        },
        "required": []
    }
`
	},
	listcomponent: {
		deno: `export async function main() {
    return [{
        "foo": 1,
    }, {
        "foo": 2,
    }, {
        "foo": 3,
    }];
}`,
		python3: `def main():
    return [{"foo": 1}, {"foo": 2}, {"foo": 3}]`
	}
} as const
