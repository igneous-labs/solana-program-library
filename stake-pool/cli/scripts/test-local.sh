#!/usr/bin/env bash

# Script to test a local solana-test-validator with the stake pool program
# Run only after `setup-local.sh` and `setup-stake-pool.sh` has completed

cd "$(dirname "$0")"
max_validators=$1
validator_list=$2

if [ "$#" -ne 2 ]; then
    echo "Expected 2 params, max_validators and validator_list"
    exit 0
fi

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