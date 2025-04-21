# {{ project-name | title_case }}

This project is a Tari template project that can contain multiple WASM template sub-projects.

# Generate new WASM template projects

1. Go to project directory
2. Run `tari new` command to generate a new WASM template (you can select from the available ones)
```sh
tari new
```

# Build binaries
To build all the wasm (WebAssembly) template binaries just run the following in the project root:
```sh
cargo build-wasm
```
or for an optimized release build:
```sh
cargo build-wasm --release
```

To build specific wasm template projects, simply pass `--package <PROJECT_NAME>`

Examples:
```sh
cargo build-wasm --package counter
```

or in case of a release build

```sh
cargo build-wasm --release --package counter
```

# Testing

To run tests simply run the usual
```sh
cargo test
```

