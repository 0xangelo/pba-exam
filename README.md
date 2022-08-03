# Polkadot Blockchain Academy - Final exam

This repository contains the implementation of the pallets and local node for the PBA's final exam. The task chosen for this assignment was [Create a simple multi-token DEX](https://hackmd.io/@fscJ0GEvRb2rqALLZB1kfA/Hk9muBgac).


## Contents

The assignment consists of the following deliverables.
- [DEX pallet](./frame/dex), built from scratch
- [Kitties NFT pallet](./frame/kitties), modified from an [existing pallet](https://github.com/substrate-developer-hub/substrate-node-template/tree/tutorials/solutions/kitties/pallets/kitties)
- [Custom node](./runtime) to integrate the DEX, Kitties, and [Assets](https://github.com/paritytech/substrate/tree/master/frame/assets) pallets

### DEX pallet

The pallet has each extrinsic documented. To get a better understanding of what order of events are expected and which scenarios may lead to errors, check the `tests.rs` file.

It also exposes a trait interface `SwapSimulation` to allow other pallets to query the price of an amount of asset via a specific AMM. This takes into account slippage and fees, so the returned price should be as close as possible to the actual input amount required to get the desired amount of asset.

### Kitties NFT pallet

This has been extended from the original Substrate kitties tutorial to handle multi-assets. Users may choose which asset to quote their NFTs in. This is made possible by loosely coupling with Substrate's `pallet-assets`.

The `tests.rs` was modified to work with the new multi-asset mechanism. The file includes an example, at the very end, of how users may use the DEX pallet to aquire the assets necessary to buy a particular NFT.

### Custom node

This is a modification of the [Substrate Node Template](https://github.com/substrate-developer-hub/substrate-node-template), incorporating the assets, dex, and kitties pallets into the runtime. The `chain_spec.rs` file was also modified so that accounts are endowed with some amount of assets to play with.

Build the node with `cargo build --release` in the root directory. After launching the node with `./target/release/node-template --dev`, one can interact with the added pallets through [Polkadot.js](https://polkadot.js.org/apps).
