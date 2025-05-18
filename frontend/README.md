### Tooling

```console
$ node --version
v22.15.0

$ npm --version
10.9.2
```

### Build servable web content

Emitted to `./out/`, configured in `./vite.config.ts`:

```console
$ export VITE_BACKEND_HOST=api.rusctl.internal:8080
$ ./node_modules/.bin/vite build
```

### Analyze circular dependencies

Using `madge` (a CLI tool distributed via _npm_).

```console
$ madge --version
8.0.0

$ madge --circular --extensions ts,tsx --image circular-deps.svg ./src/
Processed 8 files (581ms)

✔ Image created at .../circular-deps.svg
```
