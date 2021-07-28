#!/usr/bin/env bash

# Script to test a local solana-test-validator with the stake pool program
# Run only after `setup-local.sh` and `setup-stake-pool.sh` has completed
# 
# Jank setup for backward compatibility test
# 1. solana-keygen or deploy the program once to generate a stake-pool-id.json keypair file to use
#    as a consistent upgradeable program address for the stake pool program
# 2. Modify solana_program::declare_id!() in lib.rs to stake-pool-id.json's public key
# 3. Modify setup_validator() in setup-local.sh: 
#    Remove --bpf-program SPoo1xuN9wGpxNjGnPNbRPtpQ7mHgKM8d9BeFC549Jy ../../../target/deploy/spl_stake_pool.so arg
#    since that will make it unupgradeable. 
#    Deploy the program after the solana-test-validator has started to the pubkey in stake-pool-id.json using:
#    solana program deploy --program-id stake-pool.id.json ../../../target/deploy/spl_stake_pool.so
# 4. Run setup-local.sh and setup-stake-pool.sh. Test using test-local.sh.
# 5. Checkout to new updated branch. Recompile the program with cargo build-bpf --manifest-path ../../program/Cargo.toml
# 6. Redeploy the program to the same address with solana program deploy --program-id stake-pool.id.json ../../../target/deploy/spl_stake_pool.so
# 7. Run setup-stake-pool.sh and test-local.sh to verify that everything still works

if [ "$#" -ne 2 ]; then
    echo "Expected 2 args, max_validators and validator_list"
    exit 0
fi

cd "$(dirname "$0")"
max_validators=$1
validator_list=$2

# files
keys_dir=keys
spl_stake_pool=../../../target/debug/spl-stake-pool
stake_pool_keyfile=$keys_dir/stake-pool.json
mint_keyfile=$keys_dir/mint.json

# pubkeys
pool=$(solana-keygen pubkey $stake_pool_keyfile)
mint=$(solana-keygen pubkey $mint_keyfile)

test_remove_then_readd () {
  for validator in $(cat $validator_list)
  do
    $spl_stake_pool update $pool
    $spl_stake_pool remove-validator $pool $validator
  done
  for validator in $(cat $validator_list)
  do
    $spl_stake_pool update $pool
    $spl_stake_pool add-validator $pool $validator
  done
  # check all validator accounts still there
  n_validators=$($spl_stake_pool list $pool | grep "^Validator Vote Account" | wc -l)
  echo $n_validators
  if [ "$n_validators" -ne "$max_validators" ]; then
      echo "list expected $max_validators validators, got $n_validators validators"
      exit 0
  fi
}


echo "Test remove then readd validator"
test_remove_then_readd