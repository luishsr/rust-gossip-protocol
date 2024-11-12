# Use the official Rust image
FROM rust:1.70

# Set the working directory
WORKDIR /usr/src/gossip_protocol

# Copy the project files
COPY . .

# Build the Rust project
RUN cargo build --release

# Expose the port if using a direct TCP connection
EXPOSE 8080

# Run the built binary
CMD ["./target/release/gossip_protocol"]
