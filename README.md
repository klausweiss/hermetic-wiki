# hermetic

The files for installing hermetic with uv and Nix are in [`packaging/`](packaging/).

Run without installing:

```sh
uvx --from "git+ssh://git@github.com/klausweiss/hermetic-wiki.git#subdirectory=packaging" hermetic --help
```

Install:

```sh
uv tool install "git+ssh://git@github.com/klausweiss/hermetic-wiki.git#subdirectory=packaging"
```

Both build hermetic from source, so they need a Rust toolchain (`cargo`) on the machine.

## With Nix

Nix builds hermetic with its own toolchain, so nothing else needs to be installed.
These need flakes enabled (`experimental-features = nix-command flakes`).

Run without installing:

```sh
nix run "git+ssh://git@github.com/klausweiss/hermetic-wiki.git?dir=packaging" -- --help
```

Install into your profile:

```sh
nix profile install "git+ssh://git@github.com/klausweiss/hermetic-wiki.git?dir=packaging"
```

From a checkout, `nix build ./packaging` puts the binary in `./result/bin/hermetic`, and
`nix develop ./packaging` opens a shell with the tools for working on hermetic.
