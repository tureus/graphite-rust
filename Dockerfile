FROM debian:jessie

RUN apt-get update && apt-get install -y curl file libssl-dev gcc && curl -sSf https://static.rust-lang.org/rustup.sh | sh -s -- --yes --disable-sudo --channel=nightly

RUN curl -sSL https://get.docker.io/builds/Linux/x86_64/docker-1.2.0 -o /tmp/docker && \
    echo "540459bc5d9f1cac17fe8654891814314db15e77 /tmp/docker" | sha1sum -c - && \
    mv /tmp/docker /usr/local/bin/docker && \
    chmod +x /usr/local/bin/docker

ADD . /graphite-rust
WORKDIR /graphite-rust
RUN cargo build --release

ADD docker/Dockerfile.final /graphite-rust/target/release/Dockerfile

WORKDIR /graphite-rust/target/release
CMD docker build -t xrlx/graphite .
