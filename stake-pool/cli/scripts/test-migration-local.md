# Instructions for testing migrating locally:

1. Create a new keypair for the program id: `solana-keygen new --no-passphrase -s -o program.json`
2. Change `solana_program::declare_id!()` to `program.json`'s pubkey in `src/lib.rs`. You can use `solana-keygen pubkey program.json` to get the pubkey.
3. Edit `setup_validator()` in `setup-local.sh` to make the stake pool program upgradeable:
    - Delete the `--bpf-program SPoo1xuN9wGpxNjGnPNbRPtpQ7mHgKM8d9BeFC549Jy ../../../target/deploy/spl_stake_pool.so` line
    - Add `solana program deploy --program-id program.json ../../../target/deploy/spl_stake_pool.so` after `sleep 5`
4. Build and deploy the old version of the program with `setup-local.sh`
5. Launch a stake pool with the old version with `setup-stake-pool.sh`. Save the created key jsons across the various branches with git stash or whatever means.
6. Checkout the migration branch and build program with `cargo build-bpf --manifest-path ../../program/Cargo.toml`
7. Upgrade program with `solana program deploy --program-id program.json ../../../target/deploy/spl_stake_pool.so`
8. Build CLI with `cargo build --manifest-path ../Cargo.toml`
9. Run migration instruction
10. Checkout the new version
11. Build program with `cargo build-bpf --manifest-path ../../program/Cargo.toml`
12. Deploy program with `solana program deploy --program-id program.json ../../../target/deploy/spl_stake_pool.so`
13. Build cli with `cargo build --manifest-path ../Cargo.toml` and make sure everything works with `deposit-withdraw.sh` and cli. 
