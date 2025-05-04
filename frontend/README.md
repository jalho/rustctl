### Tooling

```console
$ node --version
v22.15.0

$ npm --version
10.9.2
```

### Start development server

```console
$ npm ci
$ export VITE_BACKEND_HOST=192.168.0.103:8080
$ ./node_modules/.bin/vite --host
```

### Build servable web content

Emitted to `./out/`, configured in `./vite.config.ts`:

```console
$ ./node_modules/.bin/vite build
```
