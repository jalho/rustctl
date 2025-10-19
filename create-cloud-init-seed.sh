#!/bin/bash

OUTPUT_ISO="seed.iso"
WORKDIR="cloud-init-seed-root"

rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"

cat > "$WORKDIR/user-data" <<EOF
#cloud-config
users:
  - name: foo4
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
chpasswd:
  expire: False
  list: |
    foo4:bar
ssh_pwauth: True
packages:
  - kbd
  - console-data
  - console-setup
runcmd:
  - echo "KEYMAP=fi" > /etc/vconsole.conf
  - echo 'XKBLAYOUT="fi"' > /etc/default/keyboard
  - setupcon -k --force
  - systemctl restart systemd-vconsole-setup.service || true
EOF

cat > "$WORKDIR/meta-data" <<EOF
instance-id: iid-local01
local-hostname: foo4-vbox
EOF

cloud-localds "$OUTPUT_ISO" "$WORKDIR/user-data" "$WORKDIR/meta-data"

echo "Seed ISO created: $OUTPUT_ISO"
