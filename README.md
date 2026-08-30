# Repository Structure

```
.
├── backend
│   │
│   └── Static linked native Linux executable: the web app's backend, and the
│       game server's launcher, each to be run as separate `systemd` units.
│
│       Deployment of the whole system is exactly 4 files: a single backend
│       executable and its state, and 2 files for a web frontend: HTML and CSS.
│
│       The backend executable is a Rust monolith, and the state is driven by
│       SQLite.
│
├── launcher
│   │
│   └── A library used by the `backend`: all the functionality for just running
│       the game server.
│
├── frontend
│   │
│   └── Vanilla web technologies that presumably never go out of date: HTML and
│       CSS. Minimal amount of JavaScript, and no frameworks at all. Preferably
│       no JavaScript at all either.
│
└── xtask
    │
    └── Utility commands for development and operations. E.g. `cargo xtask deploy`
        for running any tests, building a release bundle and deploying it to the
        internet.
```

# Coding Guidelines for Large Language Models (LLMs)

If you're an LLM, conform to the below guidelines when generating code.

1. Always fully qualify everything.

   Do not use `use ...;` statements. Instead, use the fully qualified
   expressions in-line.

   Motivation: any code snippet should make sense without knowledge of things
   polluted into some surrounding scope, and moving code around the project
   should be as simple as cut and paste.

   If a `use ...;` statement is crucial for readability, then it should be used
   right above the usage line in a minimum possible scope.

2. Never write comments nor documentation.

   Comments and documentation should only be written by humans with actual
   intelligence and good taste of what's worth commenting on and what's a good
   way a documenting.

   An LLM should only try its best to produce self-explanatory code that a human
   then may or may not commit with or without added comments after review.

3. Never add unsolicited logging or traces.

   Similar to comments and documentation, logs and traces etc. require human
   intelligence insight and good taste to distinguish what is worth making
   observable and what is only counterproductive noise.

   An LLM cannot know what is useful or not for a human maintainer that consumes
   the logs and traces. This is because it depends on the human maintainer's
   experience and familiarity of the implementation, deployment and its current
   use cases. Therefore an LLM should only add logging or traces when explicitly
   asked to do so with clear motivation.

4. Never add a new dependency, unless explicitly given permission to consider
   adding a new dependency. Ideally, there are no dependencies.

   A good dependency is one that focuses on one problem only, and does not
   cover other problems that are irrelevant for the project. Therefore a good
   dependency also only introduces a minimal amount of transitive dependencies,
   again ideally none.

   If a crucial dependency is explicitly allowed to be introduced into the
   project, then its version must be pinned exactly.

   Motivation: any changes to the project's code should be driven by the
   product's features' development or fixes only. No code churn should be caused
   by e.g. bloated dependencies that aren't necessarily crucial but that will
   always rot over time due to e.g. holes in security.

5. Focus on minimal scope always, and never widen the scope implicitly if not
   asked for explicitly.

   For example, do not consider cross-platform support any feature implemented
   in a web app's backend that is intended to only ever be deployed on very
   specific hardware and operating system.

   Generally speaking, a reasonable assumption is that the backend is deployed
   on x86-64 CPU, Linux kernel and a modern Ubuntu operating system.

   Motivation is the same as in avoiding unnecessary dependencies: we only want
   to be concerned with actually relevant problems.

# Other Prompt Copypasta

Miscellaneous spells that may or may not make an LLM useful.

```
Never give long explanations or verbose examples unless explicitly asked for.

Always answer in ASD-STE100 style.

Always answer in a few short sentences only unless explicitly given another
instruction.
```
