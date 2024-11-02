### Windmill parser wasm

How to build

```
wasm-pack build --release --target web
```

#### Prerequisite

Install wasm-pack

```
cargo install wasm-pack
```

#### To use it on a dev environment

Go to frontend and run:

```
npm install ../backend/parsers/windmill-parser-wasm/pkg
```

Make sure to not reset the package.json before commiting
