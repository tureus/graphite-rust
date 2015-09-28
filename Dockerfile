FROM andrewd/rust-musl

RUN curl -sSL https://get.docker.io/builds/Linux/x86_64/docker-1.2.0 -o /tmp/docker && \
    echo "540459bc5d9f1cac17fe8654891814314db15e77 /tmp/docker" | sha1sum -c - && \
    mv /tmp/docker /usr/local/bin/docker && \
    chmod +x /usr/local/bin/docker

ADD . /graphite-rust
WORKDIR /graphite-rust
RUN cargo build --release --target x86_64-unknown-linux-musl

WORKDIR /graphite-rust/target/x86_64-unknown-linux-musl/release

ADD docker/Dockerfile.final /graphite-rust/target/x86_64-unknown-linux-musl/release/Dockerfile

CMD docker build -t xrlx/graphite .
