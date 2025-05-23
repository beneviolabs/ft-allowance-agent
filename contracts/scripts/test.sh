# Run the tests with output
cargo near build --no-docker
cargo test --test unit_tests -- --nocapture
cargo test --test integration_tests -- --nocapture
