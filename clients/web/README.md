## Cheatsheet

Start backend in its root:

```
$ cargo run -- --skip -i 192.168.0.103 -p 8080
```

Start web app dev server in its root:

```
$ dx serve --platform web --addr 192.168.0.103 --port 8000
```

Now you should be able to connect to the dev server at `192.168.0.103:8000` from
some other host in the LAN.
