use starknet::accounts::ConnectedAccount;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use starknet::providers::Url;
use starknet::{
    accounts::{ExecutionEncoding, SingleOwnerAccount},
    core::types::Felt,
    providers::ProviderError,
    signers::LocalWallet,
};

/// A starknet provider encompassing a signer (starknet::LocalWallet) and account (starknet::SingleOwnerAccount)
pub struct CoNodeStarknetProvider {
    pub signer: LocalWallet,
    pub account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

impl CoNodeStarknetProvider {
    /// Create an instance of the starknet provider
    pub fn new(
        wallet: &LocalWallet,
        rpc: String,
        address: Felt,
        chain_id: Felt,
    ) -> Result<Self, ProviderError> {
        let client_and_provider = JsonRpcClient::new(HttpTransport::new(Url::parse(&rpc).unwrap()));

        let account = SingleOwnerAccount::new(
            client_and_provider,
            wallet.clone(),
            address,
            chain_id,
            ExecutionEncoding::New,
        );

        Ok(Self {
            signer: wallet.clone(),
            account,
        })
    }

    /// Returns the underlying provider
    pub fn provider(&self) -> &JsonRpcClient<HttpTransport> {
        self.account.provider()
    }
}
