### Admin CLI

The admin CLI is a simple rust CLI that can be used to deploy and interact with a SPL Token.

To run the CLI, first build it:

```bash
cargo run
```

Run help for the complete list of options:

```bash
cargo run -- --help
```

```
admin-token-cli 0.1.0


USAGE:
    admin-token-cli [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -C, --config <PATH>        Configuration file to use [default:
                               /Users/<user>/.config/solana/cli/config.yml]
    -h, --help                 Print help information
        --keypair <KEYPAIR>    Filepath or URL to a keypair [default: client keypair]
    -u, --url <URL>            JSON RPC URL for the cluster [default: value from configuration file]
    -v, --verbose              Show additional information
    -V, --version              Print version information

SUBCOMMANDS:
    createTokenWithMetadata    Creates a new token with metadata
    help                       Print this message or the help of the given subcommand(s)
    mintTo                     Mints tokens to a specific account
    transferTo                 Transfer tokens from signer to receiver
```

Example:

```bash
cargo run -- mintTo --url https://api.devnet.solana.com --keypair "usb://ledger?key=0" RECEIVER_PUBKEY MINT_PUBKEY 10000000000 9
```

To sign and send a transaction using ledger Nano S, do the following:

1. `Allow blind signing` in the ledger settings.
2. `Pubkey length` set to `Long` in the ledger settings.
