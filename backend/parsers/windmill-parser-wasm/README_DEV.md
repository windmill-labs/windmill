
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

Or enter nix devshell

```
nix develop ../../#wasm
```

#### Dev locally

```
./dev.nu <language>
```


#### Or how to use it on a dev environment manually

Go to frontend and run:

```
npm install ../backend/parsers/windmill-parser-wasm/pkg
```

Make sure to not reset the package.json before commiting

#### Testing with docker

Go to the root
```
sudo docker/dev.nu up --features "<feature1>,<feature2>" --wasm-pkg <language>
```

For example to test `nu`:
```
sudo docker/dev.nu up --features "static_frontend,nu" --wasm-pkg nu
```
