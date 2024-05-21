use {
    crate::{mint_to::process_mint_to, transfer_to::process_transfer_to},
    clap::{crate_description, crate_name, crate_version, Arg, Command},
    create_token::process_create_token,
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

pub mod create_token;
pub mod mint_to;
pub mod transfer_to;

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
                        .required(true)
                        .value_name("MINT_AUTHORITY")
                        .takes_value(true)
                        .help("Mint Authority (true/false)"),
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
                if let Some(mint_keypair) = arg_matches.get_one::<String>("mint_keypair") {
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

        _ => unreachable!(),
    }

    Ok(())
}
