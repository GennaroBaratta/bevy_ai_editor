#!/bin/bash
echo "Starting Bevy game in background..."
cd examples/simple_game
cargo run --release &
GAME_PID=$!
cd ../..

echo "Waiting 3 seconds for game to start..."
sleep 3

echo "Starting Axiom editor..."
cd apps/axiom
cargo run --release

# Kill game when editor exits
kill $GAME_PID 2>/dev/null
