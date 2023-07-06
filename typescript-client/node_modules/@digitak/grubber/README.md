**_Grubber_** is a lightweight and friendly utility to parse code with regular expressions in a 100% safe way - without having to use an AST 🐛

In a higher level, Grubber also exposes helper functions to parse the dependencies of a file in many languages (Javascript, Typescript, Css, Scss, Python, Rust, C / C++, Nim, ...).

## How?

The problem with parsing a source file with regular expressions is that you cannot be sure your match is not commented or inside a string.

For example, let's say you are looking for all `const` statements in a Javascript file - you would use a regular expression similar to:

```ts
/\bconst\s+/g;
```

But what if the file you want to parse is something like:

```ts
const x = 12;
// const y = 13;
let z = "const ";
```

Then you would match three `const` when only one should be matched.

Grubber understands what is a string, what is a comment and what is code so that you can overcome the issue very easily:

```ts
import { grub } from "@digitak/grubber";

const content = `
const x = 12
// const y = 13
let z = "const "
`;

const results = grub(content).find(/\bconst\s+/);
console.log(results.length); // will print 1 as expected
```

> For the sake of the demonstration we used a simple regex, but remember that Ecmascript is a tricky language! Effectively finding all `const` statements would require a more refined regex. Ex: `foo.const = 12` would be matched. Languages that use semi-colon at the end of every statement or strict indentation are much easier to parse in a 100% safe way.

## Installation

Use your favorite package manager:

```
npm install @digitak/grubber
```

## Grubber API

Grubber exports one main function `grub`:

```ts
export function grub(
	source: string,
	languageOrRules: LanguageName | Rule[] = "es",
): {
	// find one or more expressions and return an array of fragments
	find: (...expressions: Array<string | RegExp>) => Fragment[];

	// replace one or more expressions and return the patched string
	replace: (
		...fromTos: Array<{
			from: string | RegExp;
			to: string | RegExp;
		}>
	) => string;

	// find all dependencies (ex: `imports` in Typescript, `use` in Rust)
	findDependencies: () => Fragment[];

	// replace all dependencies by the given value
	// you can use special replace patterns like "$1" to replace
	// with the first captured group
	replaceDependencies: (to: string) => string;
};
```

The `find` and `findDependencies` methods both return an array of fragments:

```ts
export type Fragment = {
	slice: string; // the matched substring
	start: number; // start of the matched substring
	end: number; // end of the matched substring
	groups: string[] = []; // the captured groups
};
```

### Using grubber with one of the preset languages

You can use any of the preset languages:

```ts
export type LanguageName =
	| "es" // Ecmascript (Javascript / Typescript / Haxe): the default
	| "rs" // Rust
	| "css"
	| "scss"
	| "sass"
	| "c"
	| "cpp"
	| "py" // Python
	| "nim";
```

Example:

```ts
// find all semi-colons inside the rust source code
grub(rustCodeToParse, "rs").find(";");
```

### Using grubber with custom rules

You may define custom rules for the grubber parser, ie. what should be ignored an treated as "not code".

A `Rule` has the following type:

```ts
export type Rule =
	| {
			expression: string | RegExp; // the expression to ignore

			// if returns false, the match is ignored
			onExpressionMatch?: (match: RegExpExecArray) => boolean | void;
	  }
	| {
			startAt: string | RegExp; // start of the expression to ignore
			stopAt: string | RegExp; // stop of the expression to ignore

			// if returns false, the match is ignored
			onStartMatch?: (match: RegExpExecArray) => boolean | void;
			onStopMatch?: (match: RegExpExecArray) => boolean | void;
	  };
```

For example, the rules used for the `C` language are:

```ts
const rules: Rule[] = [
	{
		// string
		expression: /".*?[^\\](?:\\\\)*"/,
	},
	{
		// single line comment
		expression: /\/\/.*/,
	},
	{
		// multiline comment
		expression: /\/\*((?:.|\s)*?)\*\//,
	},
];
```

> Rules are quite simple for most languages but get complicated for Ecmascript because of the `${...}` syntax. Hopefully the job is already done for you!

🌿 🐛 🌿
