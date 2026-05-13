FROM ubuntu:24.04
RUN apt-get update && \
    apt-get install -y clang-18 llvm-18 lld-18 git valgrind && \
    rm -rf /var/lib/apt/lists/*
COPY bin/suru-build /usr/local/bin/suru-build
RUN ln -sf /usr/local/bin/suru-build /usr/local/bin/suru
COPY runtime/ /usr/local/lib/suru/runtime/
WORKDIR /work
