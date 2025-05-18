FROM rust:1.87

COPY ./ ./

RUN cargo build --release

CMD ["./target/release/nixte-server"]
