use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::MintV1Builder;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use solana_sdk::{signature::Signature, signer::Signer};
use solana_sdk::{system_program, sysvar};
use spl_associated_token_account::get_associated_token_address;

pub async fn process_mint_to(
    rpc_client: &RpcClient,
    signer: &dyn Signer,
    mint_pubkey: Pubkey,
    receiver_pubkey: Pubkey,
    amount: u64,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let receiver_ata = get_associated_token_address(&receiver_pubkey, &mint_pubkey);

    let (metadata, _) = Metadata::find_pda(&mint_pubkey);

    let mint_to_ix = MintV1Builder::new()
        .token(receiver_ata)
        .token_owner(Option::<Pubkey>::Some(receiver_pubkey))
        .metadata(metadata)
        .mint(mint_pubkey)
        .amount(amount)
        .authority(signer.pubkey())
        .payer(signer.pubkey())
        .system_program(system_program::ID)
        .sysvar_instructions(sysvar::instructions::ID)
        .spl_token_program(spl_token::ID)
        .spl_ata_program(spl_associated_token_account::ID)
        .instruction();

    let mut tx = Transaction::new_unsigned(Message::new(&[mint_to_ix], Some(&signer.pubkey())));

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
