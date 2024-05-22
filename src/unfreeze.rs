use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use solana_sdk::{signature::Signature, signer::Signer};
use spl_associated_token_account::get_associated_token_address;

pub async fn process_unfreeze_account(
    rpc_client: &RpcClient,
    signer: &dyn Signer,
    mint_pubkey: Pubkey,
    receiver_pubkey: Pubkey,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let receiver_ata = get_associated_token_address(&receiver_pubkey, &mint_pubkey);

    let unfreeze_ix = spl_token::instruction::thaw_account(
        &spl_token::ID,
        &receiver_ata,
        &mint_pubkey,
        &signer.pubkey(),
        &[&signer.pubkey()],
    )
    .unwrap();

    let mut tx = Transaction::new_unsigned(Message::new(&[unfreeze_ix], Some(&signer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {err}"))?;

    tx.try_sign(&[signer], blockhash)
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
