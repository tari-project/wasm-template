# 🎲 Guessing Game Tari Template

A simple, fun, and interactive Tari WASM template where players compete to guess a secret number!

## 🕹️ How it Works

1. 🏁 **Start Round**: The game owner starts a round by minting a unique **NFT prize** 🏆.
2. 🧐 **Make a Guess**: Players submit their guess (a number between **0 and 10**) along with their payout account 🏦.
3. 🏁 **End Round**: The owner closes the round. A random number is generated, and the prize is automatically sent to the
   winner! 🎁

## ✨ Features

- 🎟️ **NFT Rewards**: Every round has a unique prize to be won.
- 🔒 **Secure & Fair**: Uses Tari's built-in access rules and pseudo-randomness.
- 📖 **Developer Reference**: Perfect for learning cross-component calls and resource management in Tari.

## 🚀 Quick Start

Run the tests:

```bash
cargo test
```

Compile to WASM:

```bash
cargo build --target wasm32-unknown-unknown --release
# Find the generated .wasm file in target/wasm32-unknown-unknown/release/guessing_game.wasm
```

