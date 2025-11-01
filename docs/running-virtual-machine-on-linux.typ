/*
 * $ typst --version
 * typst 0.14.0 (unknown hash)
 *
 * $ typst compile docs/running-virtual-machine-on-linux.typ docs/running-virtual-machine-on-linux.pdf
 */

#let global_block_fill  = luma(230)
#let global_block_inset = 8pt

= Running a Debian Virtual Machine On Arch Linux

2025-11-01

Documentation for running a virtual machine on a system like described in the
following table:

#table(
  columns: 4,
  [*component*],           [*name*],                       [*version*],      [*notes*],
  [kernel],                [Linux],                        [6.17.6-arch1-1], [],
  [operating system (OS)], [Arch Linux],                   [],               [],
  [emulator],              [QEMU],                         [10.1.2],         [executable: `/usr/bin/qemu-x86_64`],
  [],                      [virsh],                        [11.8.0],         [executable: `/usr/bin/virsh`],
  [guest OS],              [Debian GNU/Linux],             [13 (trixie)],    [],
)

== Steps

#enum[
  Get an image for a virtual machine (VM).

  #block(fill: global_block_fill, inset: global_block_inset)[
    ```
    $ wget https://cloud.debian.org/images/cloud/trixie/20251006-2257/debian-13-nocloud-amd64-20251006-2257.qcow2
    ```
  ]

  #block(fill: global_block_fill, inset: global_block_inset)[
    ```
    $ file debian-13-nocloud-amd64-20251006-2257.qcow2
    debian-13-nocloud-amd64-20251006-2257.qcow2: QEMU QCOW Image (v3), 3221225472 bytes (v3), 3221225472 bytes
    ```
  ]
][
  Create and launch a VM.

  #block(fill: global_block_fill, inset: global_block_inset)[
    ```
    $ virt-install \
      --name debian-13 \
      --memory 4096 \
      --vcpus 2 \
      --disk ~/Downloads/debian-13-nocloud-amd64-20251006-2257.qcow2 \
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
  `console-setup`[#footnote[https://packages.debian.org/trixie/console-setup
  (accessed 2025-11-01)]]: The installer of version 1.240 includes an
  interactive keyboard layout selection.

  #colbreak()
][
  View, shutdown and restart the VM.

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
][
  Take snapshots of a shut off VM, and revert a VM to a snapshot.

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
][
  Destroy a VM.

  #block(fill: global_block_fill, inset: global_block_inset)[
    ```
    $ virsh destroy debian-13
    $ virsh undefine debian-13
    ```
  ]
]
