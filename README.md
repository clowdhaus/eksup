# eksup

See the docs at [clowdhaus.github.io/eksup](https://clowdhaus.github.io/eksup/)

## Installation

[Archives of pre-compiled binaries for `eksup` are available for Windows, macOS and Linux.](https://github.com/clowdhaus/eksup/releases)

### Homebrew (macOS and Linux)

```sh
brew install clowdhaus/taps/eksup
```

### Cargo (rust)

```sh
cargo install eksup
```

### Source

`eksup` is written in Rust, so you'll need to grab a [Rust installation](https://www.rust-lang.org/) in order to compile it.
`eksup` compiles with Rust 1.65.0 (stable) or newer. In general, `eksup` tracks the latest stable release of the Rust compiler.

To build `eksup`:

```sh
git clone https://github.com/clowdhaus/eksup
cd eksup
cargo build --release
./target/release/eksup --version
0.11.1
```

## Local Development

`eksup` uses Rust stable for production builds, but nightly for local development for formatting and linting. It is not a requirement to use nightly, but if running `fmt` you may see a few warnings on certain features only being available on nightly.

Build the project to pull down dependencies and ensure everything is setup properly:

```sh
cargo build
```

To format the codebase:

If using nightly to use features defined in [rustfmt.toml](rustfmt.toml), run the following:

```sh
cargo +nightly fmt --all
```

If using stable, run the following:

```sh
cargo fmt --all
```

To execute lint checks:

```sh
cargo clippy --all-targets --all-features
```

To run `eksup` locally for development, simply pass `eksup` commands and arguments after `cargo run --` as follows:

```sh
cargo run -- analyze --cluster <cluster> --region <region>
```

You can think of `cargo run --` as an alias for `eksup` when running locally.
Note: you will need to have access to the cluster you are analyzing. This is generally done by ensuring you have a valid `~/.kube/config` file; one can be created/updated by running:

```sh
aws eks update-kubeconfig --name <cluster> --region <region>
```

### Running Tests

To execute the tests provided, run the following from the project root directory:

```sh
cargo test --all
```
