# Secure Aggregation Protocol Implementation

This repository contains an implementation of a secure aggregation protocol that enables multiple clients to compute the sum of their private vectors while maintaining privacy. The protocol uses packed secret sharing and supports dropout resilience and malicious security.

## Overview

The system consists of two main components:
- A server that coordinates the aggregation process
- Multiple clients that hold private data and participate in the secure computation

The protocol ensures that:
- Individual client inputs remain private
- The system can tolerate client dropouts
- The system is secure against malicious clients
- The final result is the correct sum of all non-malicious client inputs

## Prerequisites

- Rust (latest stable version)
- Cargo package manager
- Unix-like operating system (for shell scripts)

## Project Structure

```
.
├── client/             # Client implementation
├── server/            # Server implementation
├── packed_secret_sharing/  # Core secret sharing implementation
├── pss/               # Packed secret sharing utilities
└── output/            # Output directory for results
```

## Building

To build the project:

```bash
cargo build
```

## Running the Protocol

### Starting the Server

```bash
cargo run [client_num] [vector_len] [dropouts] [corrupted_num] [malicious_or_not]
```

Parameters:
- `client_num`: Number of clients to expect
- `vector_len`: Length of the input vectors
- `dropouts`: Number of expected client dropouts
- `corrupted_num`: Number of corrupted clients
- `malicious_or_not`: Whether to enable malicious security (0 or 1)

### Starting the Clients

```bash
./run.sh [client_num] [vector_len]
```

Parameters:
- `client_num`: Number of clients to start
- `vector_len`: Length of the input vectors

Note: If you make changes to the client code, rebuild before running:
```bash
cargo build
```

### Cleaning Up

After finishing, you need to kill all ports and threads to free up system resources:

```bash
./killall.sh
```

This script kills processes on ports 9999 and 8888, and terminates all client processes.

## Protocol Details

The implementation uses packed secret sharing over a finite field to achieve efficient secure aggregation. The protocol consists of several phases:

1. Setup and key distribution
2. Input sharing
3. Aggregation
4. Result reconstruction

The system can handle:
- Client dropouts during the protocol execution
- Malicious clients trying to corrupt the computation
- Efficient computation through packed secret sharing

## Security Considerations

- The protocol provides information-theoretic security
- Client inputs remain private even if other clients are corrupted
- The system can detect and handle malicious behavior
- The protocol is secure against a threshold of corrupted clients

## Performance

The implementation is optimized for:
- Efficient computation using packed secret sharing
- Minimal communication overhead
- Fast reconstruction of the final result

## License

[Add your license information here]

## Contributing

[Add contribution guidelines if applicable]
