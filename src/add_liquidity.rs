use anchor_client::anchor_lang::AnchorSerialize;
use solana_client::rpc_config::RpcSendTransactionConfig;

use crate::utils::get_function_hash;

use {
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        instruction::Instruction, message::Message, signature::Signature, transaction::Transaction,
    },
};

use solana_sdk::{instruction::AccountMeta, signer::Signer, system_program};

use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;

use bridge_program::instruction as bridge_program_instructions;

pub async fn process_add_liquidity(
    rpc_client: &RpcClient,
    signer: &dyn Signer,
    program_id: Pubkey,
    amount: u64,
    mint_of_token_sent: Pubkey,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let (bridge_pda, _) = Pubkey::find_program_address(&[b"bridge_state"], &program_id);

    let vault_ata = get_associated_token_address(&bridge_pda, &mint_of_token_sent);

    let signer_ata = get_associated_token_address(&signer.pubkey(), &mint_of_token_sent);

    let method = get_function_hash("global", "add_liquidity");

    let add_liquidity = bridge_program_instructions::AddLiquidity { amount };

    let mut method_bytes = method.to_vec();

    method_bytes.append(&mut add_liquidity.try_to_vec()?);

    let ix = Instruction::new_with_bytes(
        program_id,
        &method_bytes,
        vec![
            AccountMeta::new(bridge_pda, false),
            AccountMeta::new(vault_ata, false),
            AccountMeta::new_readonly(signer.pubkey(), true),
            AccountMeta::new_readonly(mint_of_token_sent, false),
            AccountMeta::new(signer_ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
    );

    let mut tx = Transaction::new_unsigned(Message::new(&[ix], Some(&signer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {err}"))?;

    tx.try_sign(&vec![signer], blockhash)
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
