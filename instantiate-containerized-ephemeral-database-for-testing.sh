#!/bin/bash

set -xe

IMAGE="docker.io/library/postgres:18.1-trixie@sha256:1090bc3a8ccfb0b55f78a494d76f8d603434f7e4553543d6e807bc7bd6bbd17f"
NAME="postgres-rustctl"

podman rm -f $NAME 2>/dev/null

podman run -d \
    --name $NAME \
    -p 5432:5432 \
    -e POSTGRES_USER=rustctl \
    -e POSTGRES_PASSWORD=rustctl \
    -e POSTGRES_DB=postgres \
    $IMAGE

until podman exec $NAME pg_isready -U rustctl; do
    sleep 1
done

echo "PostgreSQL is ready."
