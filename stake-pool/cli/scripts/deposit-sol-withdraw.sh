#!/usr/bin/env bash

# Script to deposit sol and withdraw stakes from a pool, given stake pool public key
# and a list of validators

cd "$(dirname "$0")"
stake_pool_keyfile=$1
validator_list=$2

stake_pool_pubkey=$(solana-keygen pubkey $stake_pool_keyfile)

sol_amount=2
half_sol_amount=1
keys_dir=keys
spl_stake_pool=../../../target/debug/spl-stake-pool

deposit_sol () {
  stake_pool_pubkey=$1
  sol_amount_deposit=$2
  $spl_stake_pool deposit-sol $stake_pool_pubkey $sol_amount_deposit
}

withdraw_stakes () {
  stake_pool_pubkey=$1
  validator_list=$2
  pool_amount=$3
  for validator in $(cat $validator_list)
  do
    $spl_stake_pool withdraw-stake $stake_pool_pubkey $pool_amount --vote-account $validator
  done
}

echo "Depositing SOL into stake pool"
deposit_sol $stake_pool_pubkey $sol_amount
echo "Withdrawing stakes from stake pool"
withdraw_stakes $stake_pool_pubkey $validator_list $half_sol_amount
