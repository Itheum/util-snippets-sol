use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;

use solana_sdk::transaction::Transaction;
use solana_sdk::{signature::Signature, signer::Signer};

pub async fn process_update_authorities(
    rpc_client: &RpcClient,
    signer: &dyn Signer,
    ix: Instruction,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let mut tx = Transaction::new_unsigned(Message::new(&[ix], Some(&signer.pubkey())));

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
