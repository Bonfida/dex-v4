# Serum dex

Orderbook-based on-chain SPL token swap market

This program is intended for use to build a decentralized exchange (DEX) specialized on SPL token swaps.

## Repository

- `program` contains the code for the on-chain program
- `js` contains the code for the JS/wasm bindings for the on-chain program, an up to date npm package is available [here](https://www.npmjs.com/package/@bonfida/dex-v4)
- `cranker` contains the code for the associated cranking runtime

## Documentation

Detailed API documentation is available for the program by running `cargo doc --open` in the `program` directory.

## Testing on devnet

#### Market information

- Market Address: `Gdaxn4WkV2ZyNcMYsUWiAnmjy4YqSka4woy8ggazh4ba`
- Base Mint (6 decimals): `72m4rktxyKqWQxTnXz1rpjJ6v9RPaa6mW5Qb2aizQ8Zq`
- Quote Mint (4 decimals): `Cetq9LiKkhvQuyHRjbk1FSbbsWSCCEVvPVQ4BHCHDF3t`

#### Faucets (airdrop test tokens):

- (Add the Test USDC Mint to your wallet)
- Go to: https://www.spl-token-ui.com/#/
- Select `devnet` in the top right
- In the `Airdrop` dropdown menu select `Token Faucets`
- Select `Token Airdrop`
- Enter your address in `Token destination address`
- Enter one of the addresses below in `Faucet address`
- Enter the amount you want to receive (with decimals, max is 1k in ui amount) and click `Airdrop Tokens`

- Base faucet: `DiptCWpttbGc5y4Pb2LhxWLnkbmZUUzmEoeca55aaJfy`
- Quote faucet: `2ewphvAYknMVe55d9KgZMEw1vKTFgFUDTdyMGNcyGD1c`
