#!/bin/bash
set -e

echo "Test profile 1"
cargo test

echo "Test profile 2"
TEST_PROFILE=2 cargo test

echo "Test profile 3"
TEST_PROFILE=3 cargo test

echo "Test profile 4"
TEST_PROFILE=4 cargo test

echo "Test profile 5"
TEST_PROFILE=5 cargo test