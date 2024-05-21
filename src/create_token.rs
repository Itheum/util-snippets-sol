use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1Builder;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::message::Message;
use solana_sdk::signature::Keypair;
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;
use solana_sdk::{signature::Signature, signer::Signer};

pub async fn process_create_token(
    rpc_client: &RpcClient,
    signer: &dyn Signer,
    mint: Keypair,
    decimals: u8,
    name: String,
    symbol: String,
    uri: String,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let (metadata, _) = Metadata::find_pda(&mint.pubkey());

    let create_ix = CreateV1Builder::new()
        .metadata(metadata)
        .mint(mint.pubkey(), true)
        .authority(signer.pubkey())
        .payer(signer.pubkey())
        .update_authority(signer.pubkey(), true)
        .is_mutable(true)
        .primary_sale_happened(false)
        .name(name)
        .uri(uri)
        .symbol(symbol)
        .seller_fee_basis_points(0)
        .token_standard(mpl_token_metadata::types::TokenStandard::Fungible)
        .decimals(decimals)
        .spl_token_program(Some(spl_token::ID))
        .system_program(system_program::ID)
        .sysvar_instructions(solana_program::sysvar::instructions::ID)
        .instruction();

    let mut tx = Transaction::new_unsigned(Message::new(&[create_ix], Some(&signer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {err}"))?;

    tx.try_sign(&vec![signer, &mint], blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {err}"))?;

    let config = RpcSendTransactionConfig {
        skip_preflight: true,
        ..RpcSendTransactionConfig::default()
    };

    let signature = rpc_client
        .send_transaction_with_config(&tx, config)
        .await
        .map_err(|err| format!("error: send transaction: {err}"))?;

    Ok(signature)
}
