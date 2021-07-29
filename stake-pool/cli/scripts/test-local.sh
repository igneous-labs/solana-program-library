#!/usr/bin/env bash

# Script to test a local solana-test-validator with the stake pool program
# Run only after `setup-local.sh` and `setup-stake-pool.sh` has completed

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

cd "$(dirname "$0")"
max_validators=$1
validator_list=$2
stake_pool_program=EuftXsdVCvtTLiNN7Uzncrgod93easrkY1gwuMK9DyvE

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
epoch_duration_s=25

build_cli() {
  cargo build --manifest-path ../Cargo.toml
}

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
  # due to delay in finalizing transaction, this number may be inconsistent
  n_validators=$($spl_stake_pool list $pool | grep "^Validator Vote Account" | wc -l)
  if [ "$n_validators" -ne "$max_validators" ]; then
      echo "WARN: list expected $max_validators validators, got $n_validators validators"
  fi
}

test_deposit_and_withdraw() {
  ./deposit-withdraw.sh $stake_pool_keyfile $validator_list
}

test_deposit_sol_and_withdraw() {
  ./deposit-sol-withdraw.sh $stake_pool_keyfile $validator_list
}

get_reserve_amt() {
  pool=$1
  $spl_stake_pool update $pool
  local reserve_line=$($spl_stake_pool list $pool | grep "^Reserve Account")
  local res=$(cut -d ":" -f3 <<< $reserve_line | colrm 1 2)
  # return via reserve_lamports global
  reserve_lamports=$res
}

test_decrease_then_increase_validator_stake() {
  local decrease_amt=1 # 1 SOL
  # just do 1 validator
  local vote_acc=$(solana-keygen pubkey $keys_dir/vote_1.json)
  get_reserve_amt $pool
  local initial_reserve=$reserve_lamports
  $spl_stake_pool decrease-validator-stake $pool $vote_acc $decrease_amt
  echo "waiting 1 epoch for reserve to get balance"
  sleep $epoch_duration_s
  get_reserve_amt $pool
  local mid_reserve=$reserve_lamports
  if (( $(echo "$mid_reserve <= $initial_reserve" | bc -l) )); then
      echo "WARN: reserve did not increase after decreasing validator stake. Initial: $initial_reserve. 1 epoch after decreasing: $mid_reserve"
  fi
  $spl_stake_pool increase-validator-stake $pool $vote_acc $decrease_amt
  echo "waiting 1 epoch for reserve to get balance"
  sleep $epoch_duration_s
  get_reserve_amt $pool
  local end_reserve=$reserve_lamports
  if (( $(echo "$end_reserve >= $mid_reserve" | bc -l) )); then
      echo "WARN: reserve did not decrease after increasing validator stake. Initial: $mid_reserve. 1 epoch after decreasing: $end_reserve"
  fi
}

echo "Program info:"
solana program show $stake_pool_program

echo "Rebuilding CLI..."
build_cli

echo "Stake pool info:"
echo "address: $pool"
echo "mint: $mint"
$spl_stake_pool list $pool

# this test seems to mess up the stake pool state quite a bit...
echo "Test remove then readd validator"
test_remove_then_readd

# can only run this once since deposit-withdraw.sh does not
# generate new keys for creating new stake accounts
echo "Test deposit and withdraw"
test_deposit_and_withdraw

echo "Test deposit SOL and withdraw"
test_deposit_sol_and_withdraw

echo "Test decrease then increase validator stake"
test_decrease_then_increase_validator_stake