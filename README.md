# suru-lang
A minimalist, library-driven, general-purpose, data-oriented programming language with static typing and no garbage collection.

## Quickstart

The **only host dependency is Docker** — the whole toolchain (.NET SDK 10 +
native LLVM 20) lives in the `suru-dev` container image, built from the repo's
[`Dockerfile`](Dockerfile). No .NET SDK, no LLVM, and nothing else needs to be
installed on the host.

Build the compiler from a clean checkout with one command:

```sh
docker compose run --rm dev scripts/build
```

The first run builds the `suru-dev` image (downloads the base image and the
pinned packages) and restores NuGet packages, so it takes a few minutes; every
subsequent run reuses the cached image and the host-owned `.cache/` under the
repo. On success it compiles the solution in Release and drives the CLI `build`
subcommand.

### Other common commands

All work happens inside the container via the [`scripts/`](scripts/) wrappers
(invoked by relative path — the compose entrypoint execs its args at the
`/workspace` WORKDIR):

```sh
docker compose run --rm dev scripts/test              # run the test suite
docker compose run --rm dev scripts/run  <file.suru>  # compile + run a source file
docker compose run --rm dev scripts/ir   <file.suru>  # dump the LLVM IR for a file
```

For a long-running shell instead of one-off commands:

```sh
docker compose up -d dev            # start the service (kept alive)
docker compose exec dev bash        # open a shell in the container
docker compose down                 # stop it
```

The `dev` service runs as your host user (`${SURU_UID:-1000}:${SURU_GID:-1000}`),
so everything the build writes into the repo — `bin/`, `obj/`, and the
git-ignored `.cache/` — stays host-owned (no sudo to clean). Override the user
for a non-1000 host account:

```sh
SURU_UID=$(id -u) SURU_GID=$(id -g) docker compose run --rm dev scripts/build
```
