#!/bin/bash

echo "Cleaning and building tests..."
cargo clean
cargo test --no-run

echo "Finding and signing test binaries..."
find target/debug/deps -type f -perm +111 | while read binary; do
    # Check if it's a test binary by looking for test-related symbols
    if file "$binary" | grep -q "executable" && nm "$binary" 2>/dev/null | grep -q "test"; then
        echo "Signing: $binary"
        codesign --entitlements test.entitlements -s - "$binary" --force
        if [ $? -eq 0 ]; then
            echo "✓ Successfully signed: $binary"
        else
            echo "✗ Failed to sign: $binary"
        fi
    fi
done

echo "Running tests..."
sudo cargo test