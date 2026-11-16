### Chart and table schema

A chart or table value, as JSON Schema. Beyond it: `x` and every `y` must be columns of `data`, `y` columns hold numbers, a scatter chart's `x` column holds numbers, and a pie chart takes exactly one `y`.

```json
{"$schema":"https://json-schema.org/draft/2020-12/schema","anyOf":[{"anyOf":[{"type":"object","properties":{"chart":{"type":"object","properties":{"type":{"description":"bar or line for categories or time, pie for parts of a whole, scatter for two numeric columns","type":"string","enum":["bar","line","pie","scatter"]},"title":{"type":"string"},"data":{"description":"The rows to plot, the same shape as a table-row-object result","minItems":1,"maxItems":5000,"type":"array","items":{"description":"One row: column name to value (string, number, boolean or null)","type":"object","propertyNames":{"type":"string"},"additionalProperties":{"anyOf":[{"type":"string"},{"type":"number"},{"type":"boolean"},{"type":"null"}]}}},"x":{"description":"Column holding the category, label or date of each row (bar, line, pie), or the numeric x value (scatter)","type":"string"},"y":{"description":"Numeric column, or columns, to plot: one series each. A pie takes exactly one","anyOf":[{"type":"string"},{"minItems":1,"maxItems":8,"type":"array","items":{"type":"string"}}]},"stacked":{"description":"bar and line: stack the series","type":"boolean"},"horizontal":{"description":"bar: draw the bars horizontally","type":"boolean"},"x_label":{"type":"string"},"y_label":{"type":"string"}},"required":["type","data","x","y"],"additionalProperties":false}},"required":["chart"],"additionalProperties":false},{"type":"object","properties":{"table-row-object":{"description":"Rows as objects; the columns are the union of their keys","minItems":1,"maxItems":5000,"type":"array","items":{"description":"One row: column name to value (string, number, boolean or null)","type":"object","propertyNames":{"type":"string"},"additionalProperties":{"anyOf":[{"type":"string"},{"type":"number"},{"type":"boolean"},{"type":"null"}]}}}},"required":["table-row-object"],"additionalProperties":false},{"type":"object","properties":{"table-col":{"description":"Columns: column name to its values, all the same length","type":"object","propertyNames":{"type":"string"},"additionalProperties":{"maxItems":5000,"type":"array","items":{"anyOf":[{"type":"string"},{"type":"number"},{"type":"boolean"},{"type":"null"}]}}}},"required":["table-col"],"additionalProperties":false},{"type":"object","properties":{"table-row":{"description":"Rows as arrays; the first row holds the column names","minItems":2,"maxItems":5000,"type":"array","items":{"type":"array","items":{"anyOf":[{"type":"string"},{"type":"number"},{"type":"boolean"},{"type":"null"}]}}}},"required":["table-row"],"additionalProperties":false}]},{"type":"object","properties":{"render_all":{"description":"Several items shown one after another","minItems":1,"maxItems":10,"type":"array","items":{"anyOf":[{"type":"object","properties":{"chart":{"type":"object","properties":{"type":{"description":"bar or line for categories or time, pie for parts of a whole, scatter for two numeric columns","type":"string","enum":["bar","line","pie","scatter"]},"title":{"type":"string"},"data":{"description":"The rows to plot, the same shape as a table-row-object result","minItems":1,"maxItems":5000,"type":"array","items":{"description":"One row: column name to value (string, number, boolean or null)","type":"object","propertyNames":{"type":"string"},"additionalProperties":{"anyOf":[{"type":"string"},{"type":"number"},{"type":"boolean"},{"type":"null"}]}}},"x":{"description":"Column holding the category, label or date of each row (bar, line, pie), or the numeric x value (scatter)","type":"string"},"y":{"description":"Numeric column, or columns, to plot: one series each. A pie takes exactly one","anyOf":[{"type":"string"},{"minItems":1,"maxItems":8,"type":"array","items":{"type":"string"}}]},"stacked":{"description":"bar and line: stack the series","type":"boolean"},"horizontal":{"description":"bar: draw the bars horizontally","type":"boolean"},"x_label":{"type":"string"},"y_label":{"type":"string"}},"required":["type","data","x","y"],"additionalProperties":false}},"required":["chart"],"additionalProperties":false},{"type":"object","properties":{"table-row-object":{"description":"Rows as objects; the columns are the union of their keys","minItems":1,"maxItems":5000,"type":"array","items":{"description":"One row: column name to value (string, number, boolean or null)","type":"object","propertyNames":{"type":"string"},"additionalProperties":{"anyOf":[{"type":"string"},{"type":"number"},{"type":"boolean"},{"type":"null"}]}}}},"required":["table-row-object"],"additionalProperties":false},{"type":"object","properties":{"table-col":{"description":"Columns: column name to its values, all the same length","type":"object","propertyNames":{"type":"string"},"additionalProperties":{"maxItems":5000,"type":"array","items":{"anyOf":[{"type":"string"},{"type":"number"},{"type":"boolean"},{"type":"null"}]}}}},"required":["table-col"],"additionalProperties":false},{"type":"object","properties":{"table-row":{"description":"Rows as arrays; the first row holds the column names","minItems":2,"maxItems":5000,"type":"array","items":{"type":"array","items":{"anyOf":[{"type":"string"},{"type":"number"},{"type":"boolean"},{"type":"null"}]}}}},"required":["table-row"],"additionalProperties":false}]}}},"required":["render_all"],"additionalProperties":false}]}
```

Bar chart, one series per y column:

```json
{
  "chart": {
    "type": "bar",
    "title": "Runs per day",
    "data": [
      {
        "day": "2026-11-10",
        "success": 42,
        "failure": 3
      },
      {
        "day": "2026-11-11",
        "success": 51,
        "failure": 1
      }
    ],
    "x": "day",
    "y": [
      "success",
      "failure"
    ],
    "stacked": true
  }
}
```

Pie chart (exactly one y column):

```json
{
  "chart": {
    "type": "pie",
    "data": [
      {
        "language": "python3",
        "scripts": 18
      },
      {
        "language": "bun",
        "scripts": 25
      }
    ],
    "x": "language",
    "y": "scripts"
  }
}
```

A chart and a table together:

```json
{
  "render_all": [
    {
      "chart": {
        "type": "line",
        "data": [
          {
            "hour": "10:00",
            "p95_ms": 120
          },
          {
            "hour": "11:00",
            "p95_ms": 180
          }
        ],
        "x": "hour",
        "y": "p95_ms",
        "y_label": "ms"
      }
    },
    {
      "table-row-object": [
        {
          "hour": "10:00",
          "p95_ms": 120,
          "jobs": 310
        },
        {
          "hour": "11:00",
          "p95_ms": 180,
          "jobs": 402
        }
      ]
    }
  ]
}
```
