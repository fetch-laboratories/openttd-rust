# If you change this version, change the numbers in .github/workflows/ci-emscripten.yml (2x) too.
FROM emscripten/emsdk:4.0.23

RUN apt-get update \
    && apt-get install -y curl gcc-12 g++-12 \
    && update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-12 100 \
    && update-alternatives --install /usr/bin/g++ g++ /usr/bin/g++-12 100 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Keep equal to the channel in rust-toolchain.toml; cmake/Rust.cmake rejects any other compiler.
ARG RUST_TOOLCHAIN=1.99.0
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:$PATH
# The host tools link the host standard library and the game the wasm32-unknown-emscripten one.
# Any user (docker run -u) may write Cargo's caches.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
        --profile minimal --default-toolchain "$RUST_TOOLCHAIN" --component rustfmt,clippy --target wasm32-unknown-emscripten \
    && chmod -R a+rwX /opt/rustup /opt/cargo

COPY ports/liblzma.py /emsdk/upstream/emscripten/tools/ports/contrib/liblzma.py
