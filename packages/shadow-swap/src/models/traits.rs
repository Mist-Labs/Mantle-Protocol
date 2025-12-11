use anyhow::Result;

pub trait ChainRelayer: Send + Sync {
    fn get_merkle_root(&self) -> impl std::future::Future<Output = Result<String>> + Send;
    fn sync_source_chain_root(
        &self,
        chain_id: u32,
        root: String,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
    fn sync_dest_chain_root(
        &self,
        chain_id: u32,
        root: [u8; 32],
    ) -> impl std::future::Future<Output = Result<String>> + Send;
    fn fill_intent(
        &self,
        intent_id: &str,
        commitment: &str,
        source_chain: u32,
        token: &str,
        amount: &str,
        source_root: &str,
        merkle_path: &[String],
        leaf_index: u32,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
    fn claim_withdrawal(
        &self,
        intent_id: &str,
        nullifier: &str,
        recipient: &str,
        secret: &str,
        claim_auth: &[u8],
    ) -> impl std::future::Future<Output = Result<String>> + Send;
    fn mark_filled(
        &self,
        intent_id: &str,
        merkle_path: &[String],
        leaf_index: u32,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
    fn refund_intent(
        &self,
        intent_id: &str,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
}
