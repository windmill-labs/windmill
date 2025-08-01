# @stoplight/yaml

[![Maintainability](https://api.codeclimate.com/v1/badges/5c6d61926d8f87b38b39/maintainability)](https://codeclimate.com/github/stoplightio/yaml/maintainability) [![Test Coverage](https://api.codeclimate.com/v1/badges/5c6d61926d8f87b38b39/test_coverage)](https://codeclimate.com/github/stoplightio/yaml/test_coverage)

Useful functions when working with YAML.

- Explore the interfaces: [TSDoc](https://stoplightio.github.io/yaml)
- View the changelog: [Releases](https://github.com/stoplightio/yaml/releases)

### Installation

Supported in modern browsers and node.

```bash
# latest stable
yarn add @stoplight/yaml
```

### Usage

- **[getJsonPathForPosition](./src/getJsonPathForPosition.ts)**: Computes JSON path for given position.
- **[getLocationForJsonPath](./src/getLocationForJsonPath.ts)**: Retrieves location of node matching given JSON path.
- **[parseWithPointers](./src/parseWithPointers.ts)**: Parses YAML into JSON and also returns diagnostics as well as full ast with line information.

```ts
// basic example of parseWithPointers
import { parseWithPointers } from "@stoplight/yaml";

const result = parseWithPointers("foo: bar");

console.log(result.data); // => the {foo: "bar"} JS object
```

```ts
// basic example of getJsonPathForPosition and getLocationForJsonPath
import { getJsonPathForPosition, getLocationForJsonPath, parseWithPointers } from "@stoplight/yaml";

const result = parseWithPointers(`hello: world
address:
    street: 123`
);

const path = getJsonPathForPosition(result, { line: 2, character: 7 }); // line and character are 0-based
console.log(path); // -> ["address", "street"];

const position = getLocationForJsonPath(result, ['address']);
console.log(position.range.start); // { line: 1, character: 8 } line and character are 0-based
console.log(position.range.end); // { line: 2, character: 15 } line and character are 0-based
```

### Contributing

1. Clone repo.
2. Create / checkout `feature/{name}`, `chore/{name}`, or `fix/{name}` branch.
3. Install deps: `yarn`.
4. Make your changes.
5. Run tests: `yarn test.prod`.
6. Stage relevant files to git.
7. Commit: `yarn commit`. _NOTE: Commits that don't follow the [conventional](https://github.com/marionebl/commitlint/tree/master/%40commitlint/config-conventional) format will be rejected. `yarn commit` creates this format for you, or you can put it together manually and then do a regular `git commit`._
8. Push: `git push`.
9. Open PR targeting the `next` branch.
