
# ----- Build Stage -----

FROM rust:1 AS build

ARG TARGETARCH
ENV CROSS_CONTAINER_IN_CONTAINER=true

WORKDIR /swaparr

COPY src ./src
COPY Cargo* ./

RUN cargo install cross

RUN cross build --release --target $TARGET 

RUN mv /swaparr/target/$TARGET/release/swaparr /opt

# ----- Runtime Stage -----

FROM scratch AS runtime

COPY --from=build /opt/swaparr /

CMD ["/swaparr"]
