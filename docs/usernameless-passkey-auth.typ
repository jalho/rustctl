/*
 * Tested with typst 0.14.2 (latest as of 2026-02-01).
 */

#import "@preview/chronos:0.2.1"

#let GLOBAL_BLOCK_FILL  = rgb(50, 50, 50)
#let GLOBAL_BLOCK_INSET = 8pt
#let GLOBAL_TEXT = rgb(230, 230, 230)
#let GLOBAL_PAGE_FILL = rgb(20, 20, 20)

#set document(
  title: [Username-less Passkey Authentication]
)
#set page(fill: GLOBAL_PAGE_FILL)
#set text(fill: GLOBAL_TEXT)
#set heading(numbering: "1.1")

#title()

This document describes a web app with a username-less passkey authentication.

For context, the web app is named `rustctl`, and it's a _Rust_ (the survival
video game distributed on _Steam_) server management and observability tool.

= Authentication And Authorization

To authenticate, a web browser client first creates a passkey and associates
it with e.g. domain `rustctl.internal` in its platform store (e.g. wherever
_Windows 11_ stores passkeys for _Brave_ browser to use). Then, the client sends
the passkey's public part to the server, and the server stores the passkey. At
this point the server knows the passkey as just some passkey that is not yet
associated with any user that would be specifically authorized to do anything.
The server sets a signed cookie for the client asserting that the session is
associated with the specific passkey's public key. This process is depicted in
@fig-passkey-create-and-register-overview.

To be authorized to do something (like set some in-game parameters of the
manageable Rust game server), a client session needs to associate itself with
a Steam identity. That is, a client who already has a passkey associated with
its session must link the session with a Steam account. This is implemented
the usual OAuth-like way (call it delegated authentication or whatever) that
Steam provides. Once this is done, the client session is associated with both
a passkey and a Steam ID. The server then checks the Steam ID to determine what
the client is allowed to do when the client requests to do something.

To further emphasize, this design implies anonymous users in some sense. That
is, the passkey itself serves as an identity, and multiple different passkeys
(e.g. across multiple devices with non-synchronized passkey stores) may be later
associated with a specific Steam ID. A non-anonymous, i.e. a well defined user
(in the sense that one may be authorized to do stuff), only emerges once a Steam
ID has been associated with one or more passkeys.

The passkey specification forces to define a `name`, a `displayName` and an
`id` for a `user`, but what those really mean in this system are names and an
ID for the passkey, and not for a user (because again, the concept of a user
in this system emerges from an association with a Steam ID). Passkeys are also
associated with a credential ID, and therefore in this system each passkey
is associated with two IDs: the thing that is specified as the aforementioned
`user.id` and the thing that is referred to as credential ID. In this system,
there is not semantic difference between the two IDs despite them having
different values and being stored in different places.

#pagebreak()
#block(breakable: false)[
  #set text(size: 8pt)
  #figure(
      chronos.diagram({
        chronos._par("os", display-name: "Windows 11", color: GLOBAL_BLOCK_FILL)
        chronos._par("browser", display-name: "Brave Browser", color: GLOBAL_BLOCK_FILL)
        chronos._par("server", display-name: "Web Server", color: GLOBAL_BLOCK_FILL)

        chronos._seq("browser", "server", comment: [
          *\#1*\
          Client requests parameters for\
          creating a passkey from server.\
          This will also initiate a server\
          side stateful transaction necessary\
          to produce a secure passkey. See\
          @passkey-registration-on-server for
          more details.
        ])

        chronos._note("over", [
          *\#2*\
          Server determines what parameters\
          to use and stores a corresponding\
          transaction in its state to be\
          finalized later.
        ], pos: "server", color: GLOBAL_PAGE_FILL)

        chronos._seq("server", "browser", comment: [
          *\#3*\
          Server responds with passkey\
          creation parameters and a reference\
          to the transaction in progress.
        ])

        chronos._note("over", [
          *\#4*\
          Call the OS's passkey store for\
          creating a passkey.
        ], pos: "browser", color: GLOBAL_PAGE_FILL)

        chronos._seq("browser", "os", comment: [
          *\#5*\
          Some platform call to create the\
          passkey.
        ])

        chronos._note("over", [
          *\#6*\
          The OS creates a passkey and stores\
          in some platform store.
        ], pos: "os", color: GLOBAL_PAGE_FILL)

        chronos._seq("os", "browser", comment: [
          *\#7*\
          Return from platform.
        ])

        chronos._seq("browser", "server", comment: [
          *\#8*\
          Client sends the public key of\
          the created passkey (and the\
          transaction ID received earlier)\
          to server.
        ])

        chronos._note("over", [
          *\#9*\
          Server finds the pending\
          transaction in its state based on\
          the transaction ID, and verifies\
          the inbound public key against the\
          stored transaction, and then stores\
          the public key.

        ], pos: "server", color: GLOBAL_PAGE_FILL)

        chronos._seq("server", "browser", comment: [
          *\#10*\
          Server responds that the passkey\
          was registered successfully.
        ])
      }),
    caption: [Overview of creating a passkey.],
  ) <fig-passkey-create-and-register-overview>
]
#pagebreak()

== Passkey Registration On Server <passkey-registration-on-server>

The server-side transaction exists to make passkey registration a well-defined
challenge-response protocol rather than a mere public-key upload. When the
server initiates passkey creation, it commits to a specific set of parameters
(in particular a fresh, unpredictable challenge and the RP (Relying Party) ID
under which the credential is to be created) and records them temporarily as
a pending transaction. When the client later returns with the newly created
credential, the server uses this stored transaction to cryptographically verify
that the credential was created in direct response to its challenge, under
the expected origin and policy, and within an acceptable time window. This
prevents replay, substitution, and cross-origin attacks, and ensures that only
credentials explicitly authorized by the server become registered.
