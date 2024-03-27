//! Example of deploying a contract from Solidity code to Anvil and interacting with it.
//!
//! This repro fails with the following error:
//!
//! Anvil running at `http://localhost:42017`
//! Deployed contract at address: 0x5fbdb2315678afecb367f032d93f642f64180aa3
//! Error: server returned an error response: error code -32003: Nonce too high

use alloy::{
    network::{EthereumSigner, TransactionBuilder},
    node_bindings::Anvil,
    primitives::U256,
    providers::ProviderBuilder,
    rpc::client::RpcClient,
    signers::wallet::LocalWallet,
    sol,
};
use eyre::Result;

// Codegen from embedded Solidity code and precompiled bytecode.
sol! {
    // solc v0.8.24; solc a.sol --via-ir --optimize --bin
    #[sol(rpc, bytecode="608080604052346100155760d2908161001a8239f35b5f80fdfe60808060405260043610156011575f80fd5b5f3560e01c9081633fb5c1cb1460865781638381f58a14606f575063d09de08a146039575f80fd5b34606b575f366003190112606b575f545f1981146057576001015f55005b634e487b7160e01b5f52601160045260245ffd5b5f80fd5b34606b575f366003190112606b576020905f548152f35b34606b576020366003190112606b576004355f5500fea2646970667358221220bdecd3c1dd631eb40587cafcd6e8297479db76db6a328e18ad1ea5b340852e3864736f6c63430008180033")]
    contract Counter {
        uint256 public number;

        function setNumber(uint256 newNumber) public {
            number = newNumber;
        }

        function increment() public {
            number++;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Spin up a local Anvil node.
    // Ensure `anvil` is available in $PATH
    let anvil = Anvil::new().try_spawn()?;

    // Set up wallet
    let wallet: LocalWallet = anvil.keys()[0].clone().into();

    // Create a provider with a signer.
    let http = anvil.endpoint().parse()?;
    let provider = ProviderBuilder::new()
        // NOTE: we are using the `with_recommended_layers` layer, which includes the `ManagedNonceLayer` and `GasEstimatorLayer`.
        .with_recommended_layers()
        .signer(EthereumSigner::from(wallet))
        .on_client(RpcClient::new_http(http));

    println!("Anvil running at `{}`", anvil.endpoint());

    // Deploy the contract.
    // TODO: reports `A required key is missing: nonce`, no way to define the nonce.
    // It should not be necessary to define the nonce as we use the ManagedNonceLayer through `.with_recommended_layers()`
    // let contract = Counter::deploy(provider).await?;

    // Deploy the contract.
    let contract_builder = Counter::deploy_builder(&provider);
    let contract_address = contract_builder
        // TODO: throws `A required key is missing: nonce` without it
        // It should not be necessary to define the nonce as we use the ManagedNonceLayer through `.with_recommended_layers()`
        .nonce(0)
        // TODO: throws `missing chain id` without it, no way to define the chain id using `with_chain_id`
        .map(|mut tx| {
            tx.set_chain_id(anvil.chain_id());
            tx
        })
        .deploy()
        .await?;
    let contract = Counter::new(contract_address, &provider);

    println!("Deployed contract at address: {:?}", contract.address());

    // Set the number to 42.
    let builder = contract
        .setNumber(U256::from(42))
        // TODO: throws `A required key is missing: nonce` without it
        // TODO: throws `server returned an error response: error code -32003: Nonce too high` (incorrect)
        // It should not be necessary to define the nonce as we use the ManagedNonceLayer through `.with_recommended_layers()`
        .nonce(1)
        // TODO: throws `server returned an error response: error code -32003: Nonce too low` (correct)
        // .nonce(0)
        .map(|mut tx| {
            tx.set_chain_id(anvil.chain_id());
            tx
        });
    let receipt = builder.send().await?.get_receipt().await?;

    println!("Set number to 42: {:?}", receipt.transaction_hash);

    // ...

    Ok(())
}
