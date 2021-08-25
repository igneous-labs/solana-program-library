# Instructions for testnet migration

1. Always make sure you have enough SOL to finish the migration. Check using local validator how much an upgrade costs.
2. Checkout migration version, change `solana_program::declare_id!()` to `5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx` in `program/src/lib.rs`.
3. Build program `cargo build-bpf --manifest-path program/Cargo.toml`, and cli `cargo build --manifest-path cli/Cargo.toml`
4. `solana program deploy --program-id 5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx.json --upgrade-authority 5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx.json target/deploy/spl_stake_pool.so`
5. Run migration instruction using the CLI `spl-stake-pool migrate 5oc4nDMhYqP8dB5DW8DHtoLJpcasB19Tacu3GWAMbQAC`
6. Checkout new version, change `solana_program::declare_id!()` to `5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx` in `program/src/lib.rs`.
7. Build program `cargo build-bpf --manifest-path program/Cargo.toml`
8. `solana program deploy --program-id 5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx.json --upgrade-authority 5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx.json target/deploy/spl_stake_pool.so`
9. Build CLI `cargo build --manifest-path cli/Cargo.toml` to use to make sure everything's ok
