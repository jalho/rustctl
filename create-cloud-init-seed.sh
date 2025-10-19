#!/bin/bash

OUTPUT_ISO="seed.iso"
WORKDIR="cloud-init-seed-root"

rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"

cat > "$WORKDIR/user-data" <<EOF
#cloud-config
users:
  - name: foo
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
chpasswd:
  expire: False
  list: |
    foo:bar
ssh_pwauth: True
EOF

cat > "$WORKDIR/meta-data" <<EOF
instance-id: iid-local01
local-hostname: foo-vbox
EOF

cloud-localds "$OUTPUT_ISO" "$WORKDIR/user-data" "$WORKDIR/meta-data"

echo "Seed ISO created: $OUTPUT_ISO"
