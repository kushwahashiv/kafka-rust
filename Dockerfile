FROM clux/muslrust:stable AS builder
COPY . .
RUN cargo build --release

FROM scratch

COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/transaction .
CMD ["./transaction"]