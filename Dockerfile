ARG RUNTIME_ROOT=/runtime-root

FROM rust:latest@sha256:4a29b0db5c961cd530f39276ece3eb6e66925b59599324c8c19723b72a423615 AS app_builder
WORKDIR /app
COPY k8s-test-utils /app/k8s-test-utils
COPY src /app/src
COPY Cargo.toml /app/Cargo.toml
COPY Cargo.lock /app/Cargo.lock
RUN cargo build --release

FROM ubuntu:24.04@sha256:7a398144c5a2fa7dbd9362e460779dc6659bd9b19df50f724250c62ca7812eb3 AS os_builder
ARG RUNTIME_ROOT
RUN apt-get update && \
     apt-get upgrade -y && \
     apt-get install -y wget file golang && \
     rm -rf /var/lib/apt/lists/*
# We use chisel to slice the required libraries into the runtime image
RUN go install github.com/canonical/chisel/cmd/chisel@latest
ENV CHISEL_BIN=/root/go/bin/chisel
# On its own, chisel does not create the dpkg status file which is required for image scanners to
# create a proper SBOM. chisel-wrapper will take care of this.
RUN wget -O /usr/local/bin/chisel-wrapper https://raw.githubusercontent.com/canonical/rocks-toolbox/v1.2.0/chisel-wrapper && \
    chmod 755 /usr/local/bin/chisel-wrapper
# Select the slices we want to include in the runtime image
RUN mkdir -p $RUNTIME_ROOT/var/lib/dpkg && \
    . /etc/lsb-release && \
    chisel-wrapper \
    --generate-dpkg-status $RUNTIME_ROOT/var/lib/dpkg/status -- \
    --release ubuntu-$DISTRIB_RELEASE \
    --root $RUNTIME_ROOT \
    base-files_base \
    base-files_release-info \
    ca-certificates_data \
    libgcc-s1_libs \
    libc6_libs \
    libssl3t64_libs

FROM scratch AS runtime
ARG RUNTIME_ROOT
COPY --from=os_builder $RUNTIME_ROOT /
COPY --from=app_builder /app/target/release/idempotent-secrets /bin/idempotent-secrets
CMD ["/bin/idempotent-secrets"]
