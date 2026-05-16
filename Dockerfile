FROM ubuntu:24.04
RUN apt-get update && \
    apt-get install -y clang-18 llvm-18 lld-18 git valgrind && \
    rm -rf /var/lib/apt/lists/*
# bin/suru-build and runtime/ are bind-mounted at run time via docker-compose.yml.
# We only create the symlink here so `suru` resolves to the mounted binary.
RUN ln -sf /usr/local/bin/suru-build /usr/local/bin/suru
WORKDIR /work
