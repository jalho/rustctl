#!/bin/bash

OUTPUT_ISO="seed.iso"
WORKDIR="cloud-init-seed-root"

rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"

cat > "$WORKDIR/user-data" <<EOF
#cloud-config
users:
  - name: foo2
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
chpasswd:
  expire: False
  list: |
    foo2:bar
ssh_pwauth: True
packages:
  - kbd
  - console-data
runcmd:
  - loadkeys fi
EOF

cat > "$WORKDIR/meta-data" <<EOF
instance-id: iid-local01
local-hostname: foo2-vbox
EOF

cloud-localds "$OUTPUT_ISO" "$WORKDIR/user-data" "$WORKDIR/meta-data"

echo "Seed ISO created: $OUTPUT_ISO"

