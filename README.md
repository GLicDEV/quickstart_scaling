# quickstart_scaling

```bash
cd quickstart_scaling/
```

## Running the project locally

If you want to test your project locally, you can use the following commands:

```bash
# Compiles the Bucket canister
./first_time.sh

# Starts the replica, running in the background
dfx start --clean --background

# Deploys your canisters to the replica and generates your candid interface
dfx deploy
```

Open the link that dfx provides under **Frontend:** (e.g. quickstart_scaling_frontend: http://127.0.0.1:8000/?canisterId=rrkah-fqaaa-aaaaa-aaaaq-cai) Your link might be different!