Assuming the frontend directory as the working directory:

```console
$ dx --version
dioxus 0.6.3

$ dx serve --platform web
$ dx serve --platform web --addr 192.168.0.103 --port 8000

$ dx bundle --platform web
$ python3 -m http.server --directory ../target/dx/rustctl-frontend/release/web/public
```
