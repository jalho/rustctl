/*
 * $ typst --version
 * typst 0.14.0 (unknown hash)
 *
 * $ typst compile ./source-file.typ ./target-file.pdf
 */

#let global_block_fill  = rgb(50,50,50)
#let global_block_inset = 8pt

#set document(
  title: [Running a Debian Virtual Machine On Arch Linux]
)
#set page(fill: rgb(20, 20, 20))
#set text(fill: rgb(230, 230, 230))
#set heading(numbering: "1.1")

#title()

2025-11-01

= Overview

#table(
  columns: 4,
  [*component*], [*name*],           [*version*],      [*notes*],
  [host kernel], [Linux],            [6.17.6-arch1-1], [],
  [host OS],     [Arch Linux],       [],               [],
  [],            [QEMU],             [10.1.2],         [executable: `/usr/bin/qemu-x86_64`],
  [],            [virsh],            [11.8.0],         [executable: `/usr/bin/virsh`],
  [guest OS],    [Debian GNU/Linux], [13 (trixie)],    [],
  [],            [libguestfs],       [1.56.2],         [executable: `/usr/bin/virt-copy-in`],
)

= Setting Up a Virtual Machine (VM)

== Get an Image For a VM

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ wget https://cloud.debian.org/images/cloud/trixie/20251006-2257/debian-13-nocloud-amd64-20251006-2257.qcow2
  ```
]

The download URL can be found from https://cloud.debian.org/images/cloud
(accessed 2025-11-01).

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ file debian-13-nocloud-amd64-20251006-2257.qcow2
  debian-13-nocloud-amd64-20251006-2257.qcow2: QEMU QCOW Image (v3), 3221225472 bytes (v3), 3221225472 bytes
  ```
]

== Resize The Image From 3 GB To 32 GB

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ qemu-img create -f qcow2 -o preallocation=metadata debian-13-resized.qcow2 32G
  ```
]

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ virt-resize --expand /dev/sda1 debian-13-nocloud-amd64-20251006-2257.qcow2 debian-13-resized.qcow2
  ```
]

== Create And Launch a VM

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ virt-install \
    --name debian-13 \
    --memory 4096 \
    --vcpus 2 \
    --disk ~/Downloads/debian-13-resized.qcow2 \
    --import \
    --os-variant debian13 \
    --network user
  ```
]

View available options for the `--os-variant` argument:

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ osinfo-query os | grep debian
  ```
]

In the guest, set Finnish keyboard layout by installing
`console-setup`#footnote[https://packages.debian.org/trixie/console-setup
(accessed 2025-11-01)]: The installer of version 1.240 includes an interactive
keyboard layout selection.

#pagebreak()

== View, Shutdown And Restart a VM

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ virsh list --all
   Id   Name        State
  ---------------------------
   1    debian-13   running
  ```
]

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ virsh shutdown debian-13
  ```
]

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ virsh start debian-13
  $ virsh console debian-13
  ```
]

== Take Snapshots Of a Shut Off VM, And Revert a VM To a Snapshot

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ virsh snapshot-create-as debian-13 000_init
  $ virsh snapshot-list debian-13
  $ virsh snapshot-revert debian-13 000_init
  ```
]

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ virsh snapshot-delete debian-13 000_init
  ```
]

== Destroy a VM

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ virsh destroy debian-13
  $ virsh undefine debian-13
  ```
]

= Copying a File From Host To Guest

Install
`qemu-guest-agent`#footnote[https://packages.debian.org/trixie/qemu-guest-agent
(accessed 2025-11-01)] in the guest. An associated `systemd` service needs to be
manually started once installed.

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  # systemctl start qemu-guest-agent
  ```
]

Shut down the guest. Then, on the host, use `virt-copy-in` from package
`libguestfs` to copy a file to the guest.

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  $ virt-copy-in -d debian-13 ~/Downloads/hello-from-host-to-guest.txt /root/
  ```
]