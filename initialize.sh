#!/usr/bin/env bash

set -e

NETWORK="$1"

WASM_PATH="./target/wasm32-unknown-unknown/release/"
CLEAR_WASM=$WASM_PATH"liquidity_pool"
DEPLOYER_WASM=$WASM_PATH"liquidity_pool_deployer"
TOKEN_ID="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

case "$1" in
standalone)
  echo "Using standalone network"
  SOROBAN_NETWORK_PASSPHRASE="Standalone Network ; February 2017"
  FRIENDBOT_URL="http://localhost:8000/friendbot"
  SOROBAN_RPC_URL="http://localhost:8000/rpc"
  ;;
futurenet)
  echo "Using Futurenet network"
  SOROBAN_NETWORK_PASSPHRASE="Test SDF Future Network ; October 2022"
  FRIENDBOT_URL="https://friendbot-futurenet.stellar.org/"
  SOROBAN_RPC_URL="https://rpc-futurenet.stellar.org:443"
  ;;
testnet)
  echo "Using Testnet network"
  SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
  FRIENDBOT_URL="https://friendbot.stellar.org/"
  SOROBAN_RPC_URL="https://soroban-testnet.stellar.org:443"
  ;;
*)
  echo "Usage: $0 standalone|futurenet|testnet"
  exit 1
  ;;
esac

echo Add the $NETWORK network to cli client 
  stellar network add \
  --global $NETWORK \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE"

if !(stellar keys address clear-admin | grep admin 2>&1 >/dev/null); then
  echo Create the admin identity
  stellar keys generate clear-admin --network $NETWORK
fi

CLEAR_ADMIN_SECRET="$(stellar keys show clear-admin)"
CLEAR_ADMIN_ADDRESS="$(stellar keys address clear-admin)"

echo "Admin Public key: $CLEAR_ADMIN_ADDRESS"
echo "Admin Secret key: $CLEAR_ADMIN_SECRET"

ARGS="--network $NETWORK --source-account clear-admin"

echo Build and optimize liquidity-pool
cargo build --target wasm32-unknown-unknown --release -p liquidity-pool
stellar contract optimize --wasm $CLEAR_WASM".wasm"

echo Build and optimize liquidity-pool-deployer
cargo build --target wasm32-unknown-unknown --release -p liquidity-pool-deployer
stellar contract optimize --wasm $DEPLOYER_WASM".wasm"

echo Deploy the clear contract deployer
echo $DEPLOYER_WASM".optimized.wasm"
CONTRACT_DEPLOYER_ID="$(
  stellar contract deploy $ARGS \
  --wasm $DEPLOYER_WASM".optimized.wasm"
)"
echo "Contract deployed successfully with ID: $CONTRACT_DEPLOYER_ID"

echo Generate Vault Address
  stellar keys generate vault --network $NETWORK
  VAULT_ADDRESS="$(stellar keys address vault)"
echo "Vault Address: $VAULT_ADDRESS"

echo Install Clear Liquidity Pool Contract
SALT="$(
  stellar contract install --wasm $CLEAR_WASM".optimized.wasm" $ARGS
)"

echo Initialize Clear Liquidity Pool Contract
CLEAR_CONTRACT="$(
  stellar contract invoke --id $CONTRACT_DEPLOYER_ID $ARGS \
   -- deploy \
   --admin $CLEAR_ADMIN_ADDRESS \
   --salt $SALT \
   --token $TOKEN_ID \
   --vault $VAULT_ADDRESS
)"

echo "Clear contract deployed successfully: $CLEAR_CONTRACT"
echo "Done"