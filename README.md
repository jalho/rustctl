# `rustctl`

Work in progress.

## Development

```sh
bash instantiate-containerized-ephemeral-database-for-testing.sh
```

```sh
export POSTGRES_PASSWORD=rustctl
```

```sh
cargo run --bin backend -- service
```

```sh
podman exec postgres-rustctl psql \
  -U rustctl -d rustctl -tA \
  -c "SELECT certificate_pem FROM rustctl.tls_pem;"
```

## TODO (Roadmap)

1. FEAT-0: Use ACME to acquire a cert, deploy to Hetzner VPS

2. FEAT-1: Add Steam Authentication, and Authorization based on it
