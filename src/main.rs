use std::str::FromStr;

use crate::{authorities::process_update_authorities, update_metadata::process_update_metadata};

use add_liquidity::process_add_liquidity;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use {
    crate::{mint_to::process_mint_to, transfer_to::process_transfer_to},
    clap::{crate_description, crate_name, crate_version, Arg, Command},
    create_token::process_create_token,
    dialoguer::{Confirm, Input},
    mpl_token_metadata::{
        accounts::Metadata, instructions::UpdateMetadataAccountV2Builder, types::DataV2,
    },
    solana_clap_v3_utils::{
        input_parsers::{parse_url_or_moniker, pubkey_of},
        input_validators::{is_valid_signer, normalize_to_url_if_moniker},
        keypair::DefaultSigner,
    },
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::Keypair,
        signer::{EncodableKey, Signer},
    },
    std::{process::exit, rc::Rc},
};

pub mod add_liquidity;
pub mod authorities;
pub mod create_token;
pub mod freeze;
pub mod mint_to;
pub mod transfer_to;
pub mod unfreeze;
pub mod update_metadata;
pub mod utils;

struct Config {
    commitment_config: CommitmentConfig,
    default_signer: Box<dyn Signer>,
    json_rpc_url: String,
    verbose: bool,
    websocket_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_matches = Command::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg({
            let arg = Arg::new("config_file")
                .short('C')
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::new("keypair")
                .long("keypair")
                .value_name("KEYPAIR")
                .validator(|s| is_valid_signer(s))
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair [default: client keypair]"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::new("json_rpc_url")
                .short('u')
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .value_parser(parse_url_or_moniker)
                .help("JSON RPC URL for the cluster [default: value from configuration file]"),
        )
        .subcommand(
            Command::new("createTokenWithMetadata")
                .about("Creates a new token with metadata")
                .arg(
                    Arg::new("decimals")
                        .required(true)
                        .value_name("DECIMALS")
                        .takes_value(true)
                        .help("Token decimals"),
                )
                .arg(
                    Arg::new("name")
                        .required(true)
                        .value_name("NAME")
                        .takes_value(true)
                        .help("Name"),
                )
                .arg(
                    Arg::new("symbol")
                        .required(true)
                        .value_name("SYMBOL")
                        .takes_value(true)
                        .help("Symbol"),
                )
                .arg(
                    Arg::new("uri")
                        .required(true)
                        .value_name("URI")
                        .takes_value(true)
                        .help("URI"),
                )
                .arg(
                    Arg::new("mint_authority")
                        .value_name("MINT_AUTHORITY")
                        .takes_value(true)
                        .help("Mint Authority (leave blank to generate new keypair)"),
                ),
        )
        .subcommand(
            Command::new("mintTo")
                .about("Mints tokens to a specific account")
                .arg(
                    Arg::new("receiver_account")
                        .required(true)
                        .value_name("RECEIVER_ACCOUNT")
                        .takes_value(true)
                        .help("Receiver account"),
                )
                .arg(
                    Arg::new("mint_pubkey")
                        .required(true)
                        .value_name("MINT_PUBKEY")
                        .takes_value(true)
                        .help("Mint pubkey"),
                )
                .arg(
                    Arg::new("amount")
                        .required(true)
                        .value_name("AMOUNT")
                        .takes_value(true)
                        .help("Amount to mint"),
                ),
        )
        .subcommand(
            Command::new("transferTo")
                .about("Transfer tokens from signer to receiver")
                .arg(
                    Arg::new("receiver_account")
                        .required(true)
                        .value_name("RECEIVER_ACCOUNT")
                        .takes_value(true)
                        .help("Receiver account"),
                )
                .arg(
                    Arg::new("mint_pubkey")
                        .required(true)
                        .value_name("MINT_PUBKEY")
                        .takes_value(true)
                        .help("Mint pubkey"),
                )
                .arg(
                    Arg::new("amount")
                        .required(true)
                        .value_name("AMOUNT")
                        .takes_value(true)
                        .help("Amount to transfer"),
                ),
        )
        .subcommand(
            Command::new("freeze")
                .about("Freeze an account")
                .arg(
                    Arg::new("account")
                        .required(true)
                        .value_name("ACCOUNT")
                        .takes_value(true)
                        .help("Account to freeze"),
                )
                .arg(
                    Arg::new("mint_pubkey")
                        .required(true)
                        .value_name("MINT_PUBKEY")
                        .takes_value(true)
                        .help("Mint pubkey"),
                ),
        )
        .subcommand(
            Command::new("unfreeze")
                .about("Unfreeze an account")
                .arg(
                    Arg::new("account")
                        .required(true)
                        .value_name("ACCOUNT")
                        .takes_value(true)
                        .help("Account to unfreeze"),
                )
                .arg(
                    Arg::new("mint_pubkey")
                        .required(true)
                        .value_name("MINT_PUBKEY")
                        .takes_value(true)
                        .help("Mint pubkey"),
                ),
        )
        .subcommand(
            Command::new("updateMetadata")
                .about("Updates metadata for a token")
                .arg(
                    Arg::new("mint_pubkey")
                        .required(true)
                        .value_name("MINT_PUBKEY")
                        .takes_value(true)
                        .help("Mint pubkey"),
                ),
        )
        .subcommand(
            Command::new("updateAuthorities")
                .about("Updates authorities for a token (Mint,Freeze,Owner)")
                .arg(
                    Arg::new("mint_pubkey")
                        .required(true)
                        .value_name("MINT_PUBKEY")
                        .takes_value(true)
                        .help("Mint pubkey"),
                ),
        )
        .subcommand(
            Command::new("addToLiquidity")
                .about("Add token supply to bridge contract as liquidity")
                .arg(
                    Arg::new("amount")
                        .required(true)
                        .value_name("AMOUNT")
                        .takes_value(true)
                        .help("Amount to add"),
                )
                .arg(
                    Arg::new("mint_of_token_sent")
                        .required(true)
                        .value_name("MINT_OF_TOKEN_SENT")
                        .help("Mint of token sent"),
                )
                .arg(Arg::new("program_id").value_name("PROGRAM_ID").help(
                    "Bridge Program ID (leave blank to use default declared in program crate)",
                )),
        )
        .get_matches();

    let (command, matches) = app_matches.subcommand().unwrap();
    let mut wallet_manager: Option<Rc<RemoteWalletManager>> = None;

    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        let default_signer = DefaultSigner::new(
            "keypair",
            matches
                .value_of("keypair")
                .map(|s| s.to_string())
                .unwrap_or_else(|| cli_config.keypair_path.clone()),
        );

        let json_rpc_url = normalize_to_url_if_moniker(
            matches
                .get_one::<String>("json_rpc_url")
                .unwrap_or(&cli_config.json_rpc_url),
        );

        let websocket_url = solana_cli_config::Config::compute_websocket_url(&json_rpc_url);

        Config {
            commitment_config: CommitmentConfig::confirmed(),
            json_rpc_url,
            verbose: matches.is_present("verbose"),
            websocket_url,
            default_signer: default_signer
                .signer_from_path(matches, &mut wallet_manager)
                .unwrap_or_else(|err| {
                    eprintln!("error: {err}");
                    exit(1);
                }),
        }
    };
    solana_logger::setup_with_default("solana=info");

    if config.verbose {
        println!("JSON RPC URL: {}", config.json_rpc_url);
        println!("Websocket URL: {}", config.websocket_url);
    }

    let rpc_client =
        RpcClient::new_with_commitment(config.json_rpc_url.clone(), config.commitment_config);

    match (command, matches) {
        ("createTokenWithMetadata", arg_matches) => {
            let decimals = arg_matches.get_one::<String>("decimals").unwrap();
            let name = arg_matches.get_one::<String>("name").unwrap();
            let symbol = arg_matches.get_one::<String>("symbol").unwrap();
            let uri = arg_matches.get_one::<String>("uri").unwrap();

            let mint_keypair =
                if let Some(mint_keypair) = arg_matches.get_one::<String>("mint_authority") {
                    Keypair::read_from_file(mint_keypair).unwrap()
                } else {
                    Keypair::new()
                };

            let signature = process_create_token(
                &rpc_client,
                config.default_signer.as_ref(),
                mint_keypair,
                decimals.parse::<u8>().unwrap(),
                name.clone(),
                symbol.clone(),
                uri.clone(),
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: {err}");
                exit(1);
            });

            println!("Signature: {signature}");
        }
        ("mintTo", arg_matches) => {
            let receiver_account = pubkey_of(arg_matches, "receiver_account").unwrap();
            let mint_pubkey = pubkey_of(arg_matches, "mint_pubkey").unwrap();
            let amount = arg_matches.get_one::<String>("amount").unwrap();

            let signature = process_mint_to(
                &rpc_client,
                config.default_signer.as_ref(),
                mint_pubkey,
                receiver_account,
                amount.parse::<u64>().unwrap(),
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: {err}");
                exit(1);
            });

            println!("Signature: {signature}");
        }

        ("transferTo", arg_matches) => {
            let receiver_account = pubkey_of(arg_matches, "receiver_account").unwrap();
            let amount = arg_matches.get_one::<String>("amount").unwrap();
            let mint_pubkey = pubkey_of(arg_matches, "mint_pubkey").unwrap();

            let signature = process_transfer_to(
                &rpc_client,
                config.default_signer.as_ref(),
                mint_pubkey,
                receiver_account,
                amount.parse::<u64>().unwrap(),
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: {err}");
                exit(1);
            });

            println!("Signature: {signature}");
        }
        ("freeze", arg_matches) => {
            let account = pubkey_of(arg_matches, "account").unwrap();
            let mint_pubkey = pubkey_of(arg_matches, "mint_pubkey").unwrap();

            let signature = freeze::process_freeze_account(
                &rpc_client,
                config.default_signer.as_ref(),
                mint_pubkey,
                account,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: {err}");
                exit(1);
            });

            println!("Signature: {signature}");
        }
        ("unfreeze", arg_matches) => {
            let account = pubkey_of(arg_matches, "account").unwrap();
            let mint_pubkey = pubkey_of(arg_matches, "mint_pubkey").unwrap();

            let signature = unfreeze::process_unfreeze_account(
                &rpc_client,
                config.default_signer.as_ref(),
                mint_pubkey,
                account,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: {err}");
                exit(1);
            });

            println!("Signature: {signature}");
        }
        ("updateMetadata", arg_matches) => {
            let mint_pubkey = pubkey_of(arg_matches, "mint_pubkey").unwrap();

            let (metadata_pubkey, _) = Metadata::find_pda(&mint_pubkey);

            let data = rpc_client
                .get_account_data(&metadata_pubkey)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: {err}");
                    exit(1);
                });

            let metadata = Metadata::safe_deserialize(data.as_ref()).unwrap();

            println!("Current metadata:");
            println!("{:#?}", metadata);

            let mut data: DataV2 = DataV2 {
                name: metadata.name,
                symbol: metadata.symbol,
                uri: metadata.uri,
                seller_fee_basis_points: metadata.seller_fee_basis_points,
                creators: metadata.creators,
                collection: metadata.collection,
                uses: metadata.uses,
            };

            let mut update_metadata_builder = UpdateMetadataAccountV2Builder::new();

            update_metadata_builder.metadata(metadata_pubkey);
            update_metadata_builder.update_authority(config.default_signer.pubkey());

            let confirm: bool = Confirm::new()
                .with_prompt("Do you want to update token metadata?")
                .interact()
                .unwrap();

            if confirm {
                let name: String = Input::new()
                    .with_prompt("Name (leave blank to skip)")
                    .allow_empty(true)
                    .interact_text()
                    .unwrap();
                if !name.is_empty() {
                    data.name = name;
                }
                let symbol: String = Input::new()
                    .with_prompt("Symbol (leave blank to skip)")
                    .allow_empty(true)
                    .interact_text()
                    .unwrap();

                if !symbol.is_empty() {
                    data.symbol = symbol;
                }

                let uri: String = Input::new()
                    .with_prompt("URI (leave blank to skip)")
                    .allow_empty(true)
                    .interact_text()
                    .unwrap();

                if !uri.is_empty() {
                    data.uri = uri;
                }

                let seller_fee_basis_points: String = Input::new()
                    .with_prompt("Seller fee basis points (leave blank to skip)")
                    .allow_empty(true)
                    .interact_text()
                    .unwrap();

                if !seller_fee_basis_points.is_empty() {
                    data.seller_fee_basis_points = seller_fee_basis_points.parse::<u16>().unwrap();
                }

                update_metadata_builder.data(data.clone());

                let is_mutable: bool = Confirm::new()
                    .with_prompt("Is mutable? ")
                    .interact()
                    .unwrap();

                update_metadata_builder.is_mutable(is_mutable);
            }

            let confirm: bool = Confirm::new()
                .with_prompt("Do you want to update the update authority?")
                .interact()
                .unwrap();

            if confirm {
                let update_authority: String = Input::new()
                    .with_prompt("Update authority address")
                    .interact_text()
                    .unwrap();

                update_metadata_builder
                    .new_update_authority(Pubkey::from_str(&update_authority).unwrap());
            }

            println!("New metadata:");
            println!("{:#?}", data);

            let confirm = Confirm::new()
                .with_prompt("Proceed with update?")
                .interact()
                .unwrap();

            if confirm {
                let signature = process_update_metadata(
                    &rpc_client,
                    config.default_signer.as_ref(),
                    update_metadata_builder,
                )
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: {err}");
                    exit(1);
                });

                println!("Signature: {signature}");
            }
        }
        ("updateAuthorities", arg_matches) => {
            let mint_pubkey = pubkey_of(arg_matches, "mint_pubkey").unwrap();

            let options = vec!["Mint authority", "Freeze authority", "Owner authority"];

            let ix: Instruction;
            let choice = dialoguer::Select::new()
                .with_prompt("Choose authority to update")
                .items(&options)
                .interact()
                .unwrap();

            let options = vec!["update", "revoke"];

            match choice {
                0 => {
                    let choice = dialoguer::Select::new()
                        .with_prompt("Choose action")
                        .items(&options)
                        .interact()
                        .unwrap();

                    if choice == 0 {
                        let new_authority: String = Input::new()
                            .with_prompt("New authority address")
                            .interact_text()
                            .unwrap();

                        ix = spl_token::instruction::set_authority(
                            &spl_token::ID,
                            &mint_pubkey,
                            Some(&Pubkey::from_str(&new_authority).unwrap()),
                            spl_token::instruction::AuthorityType::MintTokens,
                            &config.default_signer.pubkey(),
                            &[&config.default_signer.pubkey()],
                        )
                        .unwrap();
                    } else {
                        ix = spl_token::instruction::set_authority(
                            &spl_token::ID,
                            &mint_pubkey,
                            None,
                            spl_token::instruction::AuthorityType::MintTokens,
                            &config.default_signer.pubkey(),
                            &[&config.default_signer.pubkey()],
                        )
                        .unwrap();
                    }
                }
                1 => {
                    let choice = dialoguer::Select::new()
                        .with_prompt("Choose action")
                        .items(&options)
                        .interact()
                        .unwrap();

                    if choice == 0 {
                        let new_authority: String = Input::new()
                            .with_prompt("New authority address")
                            .interact_text()
                            .unwrap();

                        ix = spl_token::instruction::set_authority(
                            &spl_token::ID,
                            &mint_pubkey,
                            Some(&Pubkey::from_str(&new_authority).unwrap()),
                            spl_token::instruction::AuthorityType::FreezeAccount,
                            &config.default_signer.pubkey(),
                            &[&config.default_signer.pubkey()],
                        )
                        .unwrap();
                    } else {
                        ix = spl_token::instruction::set_authority(
                            &spl_token::ID,
                            &mint_pubkey,
                            None,
                            spl_token::instruction::AuthorityType::FreezeAccount,
                            &config.default_signer.pubkey(),
                            &[&config.default_signer.pubkey()],
                        )
                        .unwrap();
                    }
                }
                2 => {
                    todo!();
                    // let new_authority: String = Input::new()
                    //     .with_prompt("New authority owner address")
                    //     .interact_text()
                    //     .unwrap();

                    // ix = spl_token::instruction::set_authority(
                    //     &spl_token::ID,
                    //     &mint_pubkey,
                    //     Some(&Pubkey::from_str(&new_authority).unwrap()),
                    //     spl_token::instruction::AuthorityType::AccountOwner,
                    //     &config.default_signer.pubkey(),
                    //     &[&config.default_signer.pubkey()],
                    // )
                    // .unwrap();
                }
                _ => unreachable!(),
            }

            let signature =
                process_update_authorities(&rpc_client, config.default_signer.as_ref(), ix)
                    .await
                    .unwrap_or_else(|err| {
                        eprintln!("error: {err}");
                        exit(1);
                    });

            println!("Signature: {signature}");
        }
        ("addToLiquidity", arg_matches) => {
            let amount = arg_matches.get_one::<String>("amount").unwrap();
            let mint_of_token_sent = pubkey_of(arg_matches, "mint_of_token_sent").unwrap();
            let program_id = if let Some(program_id) = arg_matches.get_one::<String>("program_id") {
                Pubkey::from_str(&program_id).unwrap()
            } else {
                bridge_program::ID
            };

            let signature = process_add_liquidity(
                &rpc_client,
                config.default_signer.as_ref(),
                program_id,
                amount.parse::<u64>().unwrap(),
                mint_of_token_sent,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: {err}");
                exit(1);
            });

            println!("Signature: {signature}");
        }

        _ => unreachable!(),
    }

    Ok(())
}
