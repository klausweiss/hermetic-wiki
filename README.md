# <img src="static/favicon.svg" width="32" height="32" alt=""> hermetic

hermetic turns a folder of Markdown notes into a project wiki: you edit it in the browser, then publish it as a static site.

**Demo:** https://klausweiss.github.io/hermetic-wiki (built from [`wiki/`](wiki/))

## Install and run

Install with **either** uv (builds from source, needs `cargo`):

```sh
uv tool install "git+ssh://git@github.com/klausweiss/hermetic-wiki.git#subdirectory=packaging/python"
```

**or** Nix (needs flakes, nothing else):

```sh
nix profile install "git+ssh://git@github.com/klausweiss/hermetic-wiki.git?dir=packaging/nix"
```

Then:

```sh
hermetic serve my-wiki/         # edit in the browser
hermetic build my-wiki/ site/   # render a static site
```

<details>
<summary>Try it without installing</summary>

```sh
uvx --from "git+ssh://git@github.com/klausweiss/hermetic-wiki.git#subdirectory=packaging/python" hermetic serve my-wiki/
```

```sh
nix run "git+ssh://git@github.com/klausweiss/hermetic-wiki.git?dir=packaging/nix" -- serve my-wiki/
```

</details>
