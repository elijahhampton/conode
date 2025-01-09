use conode_starknet::{
    interface::CoNodeStarknetProvider, transaction::executor::StarknetTransactionExecutor,
};
use conode_types::{
    chain::ChainConfig, crypto::ECDSASignature, traits::chain::ChainItem, work::FlattenedWork,
};
use libp2p::futures::TryFutureExt;
use serde_json::Value;
use starknet::{
    accounts::Account,
    core::{
        crypto::{ecdsa_sign, ecdsa_verify, ExtendedSignature, Signature},
        types::{
            BlockId, BlockTag, Call, ContractClass, Felt, FlattenedSierraClass, FunctionCall,
            InvokeTransactionResult, TransactionExecutionStatus,
        },
        utils::get_selector_from_name,
    },
    providers::Provider,
    signers::{LocalWallet, Signer, VerifyingKey},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{event, span, Level};
use uuid::Uuid;

use crate::error::SyncError;

// Chain events as defined in the conode protocol smart contract
pub enum ChainEvent {
    WorkSubmission,
}

impl ToString for ChainEvent {
    fn to_string(&self) -> String {
        match self {
            ChainEvent::WorkSubmission => "WorkSubmission".to_string(),
            _ => "Unsupported".to_string(),
        }
    }
}
/// Chain manager for handling Starknet blockchain interactions and transactions.
/// Manages wallet operations, contract interactions, and transaction processing.
#[derive(Clone)]
pub struct ChainContext {
    /// The Starknet wallet for transaction signing
    wallet: LocalWallet,
    /// Provider for Starknet network interactions
    chain_provider: Arc<CoNodeStarknetProvider>,
    /// Executor for Starknet transactions
    #[allow(dead_code)] // Executor implementation
    executor: StarknetTransactionExecutor,
    // Configuration for starknet
    config: ChainConfig,
    // A cached HashMap of event selectors loaded on startup
    event_selectors: HashMap<String, Felt>,

    // A cached HashMap of function selectors loaded on startup
    function_selectors: HashMap<String, Felt>,
}

impl ChainContext {
    /// Creates a new [`ChainContext`] instance with the provided wallet.
    ///
    /// # Arguments
    /// * `wallet` - LocalWallet instance for transaction signing
    ///
    /// # Returns
    /// * New ChainManager instance
    pub fn new(
        wallet: LocalWallet,
        starknet_address: Felt,
        config: ChainConfig,
    ) -> Result<Self, String> {
        let starknet_provider = CoNodeStarknetProvider::new(
            &wallet,
            config.rpc.clone(),
            starknet_address,
            Felt::from_hex(&config.chain_id).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;

        let starknet_provider = Arc::new(starknet_provider);

        Ok(Self {
            wallet,
            chain_provider: Arc::clone(&starknet_provider),
            executor: StarknetTransactionExecutor {
                chain_provider: Arc::clone(&starknet_provider),
            },
            config,
            event_selectors: HashMap::new(),
            function_selectors: HashMap::new(),
        })
    }

    pub async fn check_contract_class_selectors(&mut self) -> Result<(), SyncError> {
        let mut provider = self.starknet_provider().provider();
        let class_hash = provider
            .get_class_hash_at(
                BlockId::Number(self.config.deployed_block_num),
                Felt::from_hex(&self.config.conode_contract).unwrap(),
            )
            .map_err(|e| SyncError::Internal)
            .await?;
        let contract_class = provider
            .get_class(BlockId::Number(self.config.deployed_block_num), class_hash)
            .map_err(|_| SyncError::Internal)
            .await?;

        match contract_class {
            ContractClass::Sierra(FlattenedSierraClass {
                abi,
                sierra_program: _,
                ..
            }) => {
                let class_abi: Value =
                    serde_json::from_str(&abi).map_err(|_| SyncError::Internal)?;

                if let Value::Array(events) = &class_abi["events"] {
                    for event in events.iter() {
                        if event["name"].as_str().is_some()
                            && !self
                                .event_selectors
                                .contains_key(event["name"].as_str().unwrap())
                        {
                            self.event_selectors.insert(
                                event["name"].as_str().unwrap().to_string(),
                                get_selector_from_name(event["name"].as_str().unwrap())
                                    .map_err(|_| SyncError::Internal)?,
                            );
                        }
                    }
                };

                Ok(())
            }
            _ => Err(SyncError::Internal),
        }
    }

    pub fn selector_as_felt(&self, selector: &str) -> Option<&Felt> {
        self.event_selectors.get(selector)
    }

    /// Returns a reference to the Starknet provider.
    ///
    /// # Returns
    /// * Reference to the CoNodeStarknetProvider
    pub fn starknet_provider(&self) -> &CoNodeStarknetProvider {
        self.chain_provider.as_ref()
    }

    /// Gets the account address as a hexadecimal string.
    ///
    /// # Returns
    /// * String representation of the account address
    pub fn address(&self) -> String {
        self.chain_provider.account.address().to_hex_string()
    }

    /// Gets the account address as a Starknet Felt.
    ///
    /// # Returns
    /// * Felt representation of the account address
    pub fn address_as_felt(&self) -> Felt {
        self.chain_provider.account.address()
    }

    /// Signs data using the provided private key.
    ///
    /// # Arguments
    /// * `private_key` - Private key for signing
    /// * `data` - Data to sign
    ///
    /// # Returns
    /// * `Option<ExtendedSignature>` - Signature if successful, None otherwise
    pub async fn sign(private_key: &Felt, data: &Felt) -> Option<ExtendedSignature> {
        match ecdsa_sign(private_key, data) {
            Ok(signature) => Some(signature),
            Err(_error) => None,
        }
    }

    /// Verifies a signature against provided data and public key.
    ///
    /// # Arguments
    /// * `data` - Data that was signed
    /// * `signer_public_key` - Public key of the signer
    /// * `data_signature` - Signature to verify
    ///
    /// # Returns
    /// * `bool` - True if signature is valid
    pub fn verify(data: Felt, signer_public_key: Felt, data_signature: &ECDSASignature) -> bool {
        let signature: Signature = data_signature.to_owned().into();

        match ecdsa_verify(&signer_public_key, &data, &signature) {
            Ok(result) => result,
            Err(_error) => false,
        }
    }

    /// Gets the public key from the wallet.
    ///
    /// # Returns
    /// * Result containing the VerifyingKey
    pub async fn pub_key(&self) -> Result<VerifyingKey, starknet::signers::Infallible> {
        self.wallet.get_public_key().await
    }

    /// Checks the ERC20 token allowance for the contract.
    ///
    /// # Arguments
    /// * `token_address` - Address of the ERC20 token
    ///
    /// # Returns
    /// * Result containing the allowance amount as Felt
    async fn check_allowance(
        &self,
        token_address: Felt,
    ) -> Result<Felt, Box<dyn std::error::Error>> {
        let selector = get_selector_from_name("allowance")?;
        let owner = self.chain_provider.account.address();
        let spender = Felt::from_hex(&self.config.conode_contract).unwrap();

        let mut calldata = Vec::new();
        calldata.push(owner);
        calldata.push(spender);

        let function_call = FunctionCall {
            contract_address: token_address,
            entry_point_selector: selector,
            calldata,
        };

        let result = self
            .chain_provider
            .provider()
            .call(function_call, BlockId::Tag(BlockTag::Latest))
            .await
            .map_err(|e| format!("Failed to check allowance: {}", e))?;

        // ERC20 returns allowance as u256 (two felts: low, high)
        match result.len() {
            2 => Ok(result[0]), // Return low bits for comparison
            _ => Err("Invalid allowance response".into()),
        }
    }

    /// Creates a new work item on the blockchain.
    ///
    /// # Arguments
    /// * `work` - Work item to create
    ///
    /// # Returns
    /// * Result containing transaction result or error
    #[allow(deprecated)]
    pub async fn create_task(
        &self,
        work: &FlattenedWork,
    ) -> Result<InvokeTransactionResult, Box<dyn std::error::Error>> {
        let create_work_tx_span = span!(Level::TRACE, "ChainManager::create_work");
        let _span = create_work_tx_span.enter();

        let contract_address = Felt::from_hex(&self.config.conode_contract).unwrap();
        let payment_token_contract_address =
            Felt::from_hex(&self.config.payment_token_contract).unwrap();

        event!(Level::TRACE, "creating calldata");
        let calldata = work
            .to_calldata()
            .map_err(|e| format!("Calldata conversion failed: {}", e))?;

        event!(Level::TRACE, "assigning selector");
        let selector = get_selector_from_name("create_work")
            .map_err(|e| format!("Selector creation failed: {}", e))?;

        let function_call = FunctionCall {
            contract_address,
            entry_point_selector: selector,
            calldata: calldata.clone(),
        };

        let execution_call = Call {
            to: contract_address,
            selector,
            calldata,
        };

        event!(Level::TRACE, "simulating transaction result");
        let simulate_result = self
            .chain_provider
            .provider()
            .call(function_call, BlockId::Tag(BlockTag::Latest))
            .await;

        match &simulate_result {
            Ok(_) => println!("Dry run successful"),
            Err(e) => println!("Dry run failed: {}", e),
        }

        event!(Level::TRACE, "approving coin quantity for transferFrom");
        let mut approve_calldata = Vec::new();
        approve_calldata.push(contract_address);
        approve_calldata.push(Felt::from(1000000_u64)); // amount low - much higher than needed
        approve_calldata.push(Felt::from(0_u64));

        let call = Call {
            to: payment_token_contract_address,
            selector: get_selector_from_name("approve").unwrap(),
            calldata: approve_calldata,
        };

        event!(Level::TRACE, "executing approve");
        let approve_receipt = self
            .chain_provider
            .account
            .execute(vec![call])
            .send()
            .await
            .map_err(|e| format!("Approve transaction failed: {}", e))?;

        // Wait for approval to be mined
        self.wait_for_tx(approve_receipt.transaction_hash).await?;

        let allowance_after = self.check_allowance(payment_token_contract_address).await?;

        assert!(
            allowance_after > Felt::from(0_u64),
            "Approval did not succeed"
        );

        if allowance_after <= Felt::from(0_u64) {
            panic!("Approval succeeded, proceeding with create_work...");
        }

        event!(Level::TRACE, "executing create_work transaction");
        let receipt = self
            .chain_provider
            .account
            .execute_v3(vec![execution_call])
            .send()
            .await
            .map_err(|e| format!("Transaction execution failed: {}", e))?;

        Ok(receipt)
    }

    /// Submits a solution for a work item.
    ///
    /// # Arguments
    /// * `work_id` - ID of the work item
    /// * `solution_hash` - Hash of the solution
    ///
    /// # Returns
    /// * Result indicating success or error
    pub async fn submit_solution(
        &self,
        work_id: String,
        solution_hash: Felt,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let contract_address = Felt::from_hex(&self.config.conode_contract).unwrap();

        // Convert the work_id (String -> Uuid -> u128 -> Felt)
        let uuid = Uuid::parse_str(&work_id).unwrap();
        let uuid_int = uuid.as_u128();
        let id_felt = Felt::from(uuid_int);
        let calldata = vec![id_felt, solution_hash];

        let selector = get_selector_from_name("submit")
            .map_err(|e| format!("Selector creation failed: {}", e))?;

        let call = Call {
            to: contract_address,
            selector,
            calldata,
        };

        let receipt = self
            .chain_provider
            .account
            .execute_v3(vec![call])
            .send()
            .await
            .map_err(|e| format!("Transaction execution failed: {}", e))?;

        self.wait_for_tx(receipt.transaction_hash).await?;

        Ok(())
    }

    /// Verifies and completes a work item with its solution.
    ///
    /// # Arguments
    /// * `work_id` - ID of the work item
    /// * `solution_hash` - Hash of the solution
    ///
    /// # Returns
    /// * Result indicating success or error
    pub async fn verify_and_complete(
        &self,
        work_id: String,
        solution_hash: Felt,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let contract_address = Felt::from_hex(&self.config.conode_contract).unwrap();

        let uuid = Uuid::parse_str(&work_id).unwrap();
        let uuid_int = uuid.as_u128();
        let id_felt = Felt::from(uuid_int);
        let calldata = vec![id_felt, solution_hash];

        let selector = get_selector_from_name("verify_and_complete")
            .map_err(|e| format!("Selector creation failed: {}", e))?;

        let call = Call {
            to: contract_address,
            selector,
            calldata,
        };

        let receipt = self
            .chain_provider
            .account
            .execute_v3(vec![call])
            .send()
            .await
            .map_err(|e| format!("Transaction execution failed: {}", e))?;

        self.wait_for_tx(receipt.transaction_hash).await?;

        Ok(())
    }

    /// Waits for a transaction to be confirmed on the blockchain.
    ///
    /// # Arguments
    /// * `tx_hash` - Hash of the transaction to wait for
    ///
    /// # Returns
    /// * Result indicating success or error
    async fn wait_for_tx(&self, tx_hash: Felt) -> Result<(), Box<dyn std::error::Error>> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 50;
        const DELAY_MS: u64 = 2000;

        while attempts < MAX_ATTEMPTS {
            match self
                .chain_provider
                .provider()
                .get_transaction_receipt(tx_hash)
                .await
            {
                Ok(receipt) => {
                    if receipt.receipt.execution_result().status()
                        == TransactionExecutionStatus::Succeeded
                    {
                        return Ok(());
                    }
                }
                Err(_) => {
                    // Transaction not yet available, continue waiting
                }
            }

            tokio::time::sleep(Duration::from_millis(DELAY_MS)).await;
            attempts += 1;
        }

        Err("Transaction not confirmed after maximum attempts".into())
    }
}
