mod want_list;

use crate::want_list::WantList;
use clap::{CommandFactory, Parser, Subcommand};
use dialoguer::{Input, Select};
use ootle_rs::{
    ToAccountAddress, TransactionRequest,
    builtin_templates::{UnsignedTransactionBuilder, faucet::IFaucet},
    key_provider::PrivateKeyProvider,
    keys::{HasViewOnlyKeySecret, OotleSecretKey},
    provider::{IndexerProvider, Provider, ProviderBuilder, WalletProvider},
    wallet::OotleWallet,
};
use random_name::generate_name;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tari_crypto::ristretto::RistrettoSecretKey;
use tari_ootle_common_types::displayable::Displayable;
use tari_ootle_common_types::{Network, engine_types::transaction_receipt::TransactionReceipt};
use tari_ootle_transaction::{TransactionBuilder, args};
use tari_template_lib_types::{
    ComponentAddress, NonFungibleAddress, NonFungibleId, ResourceAddress, TemplateAddress,
    constants::{ONE_XTR, XTR},
};
use tari_utilities::{ByteArray, hex::Hex};

// ──────────────────────────────────── CLI ────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "guessing-game-cli",
    about = "🎮 A CLI for the Tari Guessing Game",
    long_about = "This CLI allows you to interact with a Guessing Game template on the Tari network.\n\n\
                  The game flow is as follows:\n\
                  1. Run `init` to set up your admin wallet and create player accounts.\n\
                  2. Run `create` to deploy a new GuessingGame component instance.\n\
                  3. Run `start-game` with an NFT ID to begin a new round.\n\
                  4. Players use `guess` to submit their numbers (0-10).\n\
                  5. Run `end-game` to conclude the round and pay out the winner."
)]
struct Cli {
    /// Path to JSON state file
    #[arg(long, default_value = "./guessing-game-state.json")]
    state: PathBuf,

    /// Indexer REST API URL (overrides value stored in state)
    #[arg(long)]
    indexer: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize wallet, create account, and fund via faucet
    Init,
    /// Deploy a new GuessingGame component using the template address from state
    Create,
    /// Start a new round with the given NFT id
    StartGame {
        /// A unique (for the game resource) NFT id string, e.g. "round-1" or "🚀"
        nft_id: String,
    },
    /// Submit a guess (0–10)
    Guess {
        /// The number to guess (0–10)
        number: Option<u8>,
    },
    /// End the current round and pay out winner
    EndGame,
    /// Add a player who can submit guesses
    AddUser,
    /// Show the current state
    Show,
}

// ─────────────────────────────────── State ───────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct User {
    name: String,
    account_secret_hex: String,
    view_secret_hex: String,
    /// On-chain account component address, stored as "component_<hex>"
    account_address: ComponentAddress,
}

impl User {
    fn to_wallet(&self, network: Network) -> OotleWallet {
        let acc_bytes =
            Vec::from_hex(&self.account_secret_hex).expect("Invalid account secret hex");
        let view_bytes = Vec::from_hex(&self.view_secret_hex).expect("Invalid view secret hex");
        let acc_sk = RistrettoSecretKey::from_canonical_bytes(&acc_bytes)
            .expect("Invalid account key bytes");
        let view_sk =
            RistrettoSecretKey::from_canonical_bytes(&view_bytes).expect("Invalid view key bytes");
        let secret = OotleSecretKey::new(network, acc_sk, view_sk);
        OotleWallet::from(PrivateKeyProvider::new(secret))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct PlayerGuess {
    player_name: String,
    guess: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Round {
    /// The prize NFT address for this round, stored as a NonFungibleAddress string
    prize: String,
    /// Guesses submitted by players this round
    guesses: Vec<PlayerGuess>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct State {
    account_secret_hex: Option<String>,
    view_secret_hex: Option<String>,
    #[serde(default = "default_network")]
    network: String,
    #[serde(default = "default_indexer_url")]
    indexer_url: String,
    /// On-chain account component address, stored as "component_<hex>"
    account_address: Option<ComponentAddress>,
    /// Stored as "template_<hex>" string
    template_address: Option<String>,
    resource_address: Option<ResourceAddress>,
    /// GuessingGame component address, stored as "component_<hex>"
    component_address: Option<ComponentAddress>,
    #[serde(default)]
    users: Vec<User>,
    /// The current active round, populated when a game is started and cleared when it ends
    #[serde(default)]
    current_round: Option<Round>,
}

fn default_network() -> String {
    "Esmeralda".to_string()
}
fn default_indexer_url() -> String {
    "http://127.0.0.1:12500".to_string()
}

fn default_indexer_url_for_network(network: &str) -> &'static str {
    match network {
        "Esmeralda" => "http://217.182.93.35:50124/",
        _ => "http://127.0.0.1:12500",
    }
}

impl State {
    fn is_initialized(&self) -> bool {
        self.account_secret_hex.is_some() && self.view_secret_hex.is_some()
    }
}

fn load_state(path: &Path) -> anyhow::Result<State> {
    if path.exists() {
        let s = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&s)?)
    } else {
        Ok(State::default())
    }
}

fn save_state(path: &Path, state: &State) -> anyhow::Result<()> {
    let s = serde_json::to_string_pretty(state)?;
    std::fs::write(path, s)?;
    Ok(())
}

// ──────────────────────────────── Address helpers ────────────────────────────

fn parse_stored_template_address(s: &str) -> anyhow::Result<TemplateAddress> {
    use std::str::FromStr;
    let hex = s.strip_prefix("template_").unwrap_or(s);
    TemplateAddress::from_str(hex).map_err(|e| anyhow::anyhow!("Invalid template address: {e}"))
}

fn parse_network(s: &str) -> anyhow::Result<Network> {
    match s {
        "LocalNet" => Ok(Network::LocalNet),
        "Esmeralda" => Ok(Network::Esmeralda),
        "Igor" => Ok(Network::Igor),
        "MainNet" => anyhow::bail!("MainNet is not supported in this example."),
        other => anyhow::bail!("Unknown network: {other}"),
    }
}

// ──────────────────────────────── Wallet helpers ─────────────────────────────

fn wallet_from_state(state: &State) -> anyhow::Result<OotleWallet> {
    let network = parse_network(&state.network)?;
    let acc_bytes = Vec::from_hex(state.account_secret_hex.as_deref().unwrap_or(""))
        .map_err(|e| anyhow::anyhow!("Invalid account secret hex: {e}"))?;
    let view_bytes = Vec::from_hex(state.view_secret_hex.as_deref().unwrap_or(""))
        .map_err(|e| anyhow::anyhow!("Invalid view secret hex: {e}"))?;
    let acc_sk = RistrettoSecretKey::from_canonical_bytes(&acc_bytes)
        .map_err(|e| anyhow::anyhow!("Invalid account key: {e}"))?;
    let view_sk = RistrettoSecretKey::from_canonical_bytes(&view_bytes)
        .map_err(|e| anyhow::anyhow!("Invalid view key: {e}"))?;
    let secret = OotleSecretKey::new(network, acc_sk, view_sk);
    Ok(OotleWallet::from(PrivateKeyProvider::new(secret)))
}

// ──────────────────────────────── Transaction helpers ────────────────────────

async fn build_and_send(
    provider: &mut IndexerProvider<OotleWallet>,
    build_fn: impl FnOnce(TransactionBuilder) -> TransactionBuilder,
    want_list: WantList,
) -> anyhow::Result<TransactionReceipt> {
    let network = provider.network();

    let base_builder = TransactionBuilder::new(network).with_auto_fill_inputs();
    let unsigned_tx = build_fn(base_builder).build_unsigned();
    let unsigned_tx = provider
        .resolve_input_want_list(unsigned_tx, want_list.items())
        .await?;

    let tx = TransactionRequest::default()
        .with_transaction(unsigned_tx)
        .build(provider.wallet())
        .await?;

    let pending = provider.send_transaction(tx).await?;
    println!("⏳ Transaction submitted: {}", pending.tx_id());
    let outcome = pending.watch().await?;
    println!("🏁 Outcome: {outcome}");

    if let Some(reason) = outcome.reject_reason() {
        anyhow::bail!("❌ Transaction rejected: {reason}");
    }
    let receipt = pending.get_receipt().await?;
    Ok(receipt)
}

// ─────────────────────────────────── Main ────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let state_path = cli.state;
    let mut state = load_state(&state_path)?;

    // Allow --indexer flag to override stored value
    if let Some(url) = cli.indexer {
        state.indexer_url = url;
    }

    match cli.command {
        Commands::Init => cmd_init(&state_path, &mut state).await?,
        Commands::Create => cmd_create(&state_path, &mut state).await?,
        Commands::StartGame { nft_id } => cmd_start_game(&state_path, &mut state, &nft_id).await?,
        Commands::Guess { number } => cmd_guess(&state_path, &mut state, number).await?,
        Commands::EndGame => cmd_end_game(&mut state).await?,
        Commands::AddUser => cmd_add_user(&state_path, &mut state).await?,
        Commands::Show => cmd_show(&state).await?,
    }

    Ok(())
}

// ──────────────────────────────────── init ───────────────────────────────────

async fn cmd_init(state_path: &Path, state: &mut State) -> anyhow::Result<()> {
    // If fully complete, just show current state.
    if state.is_initialized() && state.account_address.is_some() {
        println!("✅ Cli is already initialized.");
        println!("  🌐 Network:          {}", state.network);
        println!("  🔗 Indexer:          {}", state.indexer_url);
        println!(
            "  🏦 Account address:  {}",
            state.account_address.as_ref().display()
        );
        println!(
            "  📄 Template address: {}",
            state
                .template_address
                .as_deref()
                .unwrap_or("(not set — run `create`)")
        );
        println!(
            "  🎮 Game component:   {}",
            state
                .component_address
                .as_ref()
                .map(|a| a.to_string())
                .unwrap_or_else(|| "(not set — run `create`)".to_string())
        );
        println!();
        Cli::command().print_help()?;
        println!();
        return Ok(());
    }

    // ── Interactive prompts (skipped if keys already exist) ──────────────────

    if !state.is_initialized() {
        let networks = &["Esmeralda", "LocalNet"];
        let network_idx = Select::new()
            .with_prompt("Select network")
            .default(0)
            .items(networks)
            .interact()?;
        let network_str = networks[network_idx];
        let network = parse_network(network_str)?;

        let indexer_url: String = Input::new()
            .with_prompt("Indexer REST API URL")
            .default(default_indexer_url_for_network(network_str).to_string())
            .interact_text()?;

        let template_input: String = Input::new()
            .with_prompt("Guessing Game template address (leave blank to set later)")
            .allow_empty(true)
            .interact_text()?;
        let template_address = if template_input.trim().is_empty() {
            None
        } else {
            let addr = parse_stored_template_address(template_input.trim())?;
            Some(format!("template_{addr}"))
        };

        let secret = OotleSecretKey::random(network);
        state.account_secret_hex = Some(secret.account_secret().to_hex());
        state.view_secret_hex = Some(secret.view_only_secret().to_hex());
        state.network = network_str.to_string();
        state.indexer_url = indexer_url;
        state.template_address = template_address;

        // Save early so key material is not lost if the next step fails
        save_state(state_path, state)?;
    } else {
        println!("🔑 Keys found. Retrying account creation and funding...");
    }

    // ── Account creation and funding ─────────────────────────────────────────

    let wallet = wallet_from_state(state)?;
    let indexer_url = state.indexer_url.clone();

    let mut provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(&indexer_url)
        .await?;

    let account_addr = provider.default_signer_address().to_account_address();

    println!("🚰 Creating account and funding via faucet...");
    println!(
        "  📬 OotleAddress (share this to receive funds): {}",
        provider.default_signer_address()
    );
    println!("  🏦 Account component: {account_addr}");

    let unsigned_tx = IFaucet::new(&provider)
        .take_faucet_funds(10 * ONE_XTR)
        .pay_fee(500u64)
        .prepare()
        .await?;

    let tx = TransactionRequest::default()
        .with_transaction(unsigned_tx)
        .build(provider.wallet())
        .await?;

    let pending = provider.send_transaction(tx).await?;
    println!("⏳ Transaction submitted: {}", pending.tx_id());
    pending.watch().await?;
    println!("💰 Account funded successfully.");

    state.account_address = Some(account_addr);
    save_state(state_path, state)?;

    // ── Create players ───────────────────────────────────────────────────────

    let num_users: u64 = Input::new()
        .with_prompt("How many game players to create?")
        .default(5)
        .interact_text()?;

    drop(provider); // release the connection before creating players

    println!("👥 Creating {num_users} player(s)...");
    // TODO: do this in one transaction
    for _ in 0..num_users {
        create_user(state_path, state, None).await?;
    }

    println!();
    println!("🎉 Initialization complete!");
    println!(
        "  📬 OotleAddress:     {}",
        state.account_address.as_ref().display()
    );
    println!("  🏦 Account address:  {account_addr}");
    println!("  👥 Players created:  {}", state.users.len());
    println!();
    println!("👉 Next step: run `create` to deploy a game component.");

    Ok(())
}

// ─────────────────────────────────── create ──────────────────────────────────

async fn cmd_create(state_path: &Path, state: &mut State) -> anyhow::Result<()> {
    if !state.is_initialized() {
        anyhow::bail!("Wallet not initialized. Run `init` first.");
    }

    let template_addr =
        parse_stored_template_address(state.template_address.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "No template address in state. Set one during `init` or update the state file."
            )
        })?)?;
    let wallet = wallet_from_state(state)?;
    let indexer_url = state.indexer_url.clone();

    let mut provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(&indexer_url)
        .await?;

    let account_addr = provider.default_signer_address().to_account_address();
    let want_list = WantList::new().add_vault_for_resource(account_addr, XTR, true);

    println!("🚀 Creating a new GuessingGame component...");
    let receipt = build_and_send(
        &mut provider,
        |builder| {
            builder
                .pay_fee_from_component(account_addr, 2000u64)
                .allocate_component_address("new_component")
                .call_function(template_addr, "new", args![Workspace("new_component")])
        },
        want_list,
    )
    .await?;

    let component_addr = receipt
        .diff_summary
        .upped
        .iter()
        .find_map(|s| s.substate_id.as_component_address())
        .ok_or_else(|| anyhow::anyhow!("No component address in receipt"))?;

    let resource_addr = receipt
        .diff_summary
        .upped
        .iter()
        .find_map(|s| s.substate_id.as_resource_address().filter(|a| *a != XTR))
        .ok_or_else(|| anyhow::anyhow!("No resource address in receipt"))?;

    println!(
        "🚀 Game created: {component_addr} (saved to {})",
        state_path.display()
    );

    state.template_address = Some(format!("template_{template_addr}"));
    state.component_address = Some(component_addr);
    state.resource_address = Some(resource_addr);

    save_state(state_path, state)?;

    println!(
        "👉 Next step: run `start-game <nft-id>` to start a new round with an NFT id of your choice."
    );

    Ok(())
}

// ────────────────────────────────── start-game ───────────────────────────────

async fn cmd_start_game(
    state_path: &Path,
    state: &mut State,
    nft_id_arg: &str,
) -> anyhow::Result<()> {
    if !state.is_initialized() {
        anyhow::bail!("Wallet not initialized. Run `init` first.");
    }

    let component_addr = state
        .component_address
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No component address in state. Run `create` first."))?;

    let wallet = wallet_from_state(state)?;
    let indexer_url = state.indexer_url.clone();

    let mut provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(&indexer_url)
        .await?;

    let account_addr = provider.default_signer_address().to_account_address();
    let resource_address = state
        .resource_address
        .ok_or_else(|| anyhow::anyhow!("No resource address in state. Run `create` first."))?;
    let want_list = WantList::new()
        .add_vault_for_resource(account_addr, XTR, true)
        .add_vault_for_resource(*component_addr, resource_address, true)
        .add_specific_substate(resource_address, true);

    let nft_id = NonFungibleId::from_string(nft_id_arg);

    println!(
        "🎲 Starting a new game round with NFT ID: {}...",
        nft_id_arg
    );
    let receipt = build_and_send(
        &mut provider,
        |builder| {
            builder
                .pay_fee_from_component(account_addr, 2000u64)
                .call_method(*component_addr, "start_game", args![nft_id])
        },
        want_list,
    )
    .await?;

    let prize_addr = receipt
        .diff_summary
        .upped
        .iter()
        .find_map(|s| {
            s.substate_id
                .as_non_fungible_address()
                .filter(|a| *a.resource_address() == resource_address)
        })
        .ok_or_else(|| anyhow::anyhow!("No prize resource address in receipt"))?;

    state.current_round = Some(Round {
        prize: prize_addr.to_string(),
        guesses: Vec::new(),
    });
    save_state(state_path, state)?;

    println!("🎲 Game started with NFT id: {nft_id_arg}");
    println!("👉 Next step: run `guess` to submit a guess (0–10).");

    Ok(())
}

// ──────────────────────────────────── guess ──────────────────────────────────

async fn cmd_guess(state_path: &Path, state: &mut State, number: Option<u8>) -> anyhow::Result<()> {
    let network = parse_network(&state.network)?;
    if !state.is_initialized() {
        anyhow::bail!("Wallet not initialized. Run `init` first.");
    }

    if state.users.is_empty() {
        anyhow::bail!(
            "No users found in state. Add a user with `add-user` before submitting guesses."
        );
    }
    let resource_address = state
        .resource_address
        .ok_or_else(|| anyhow::anyhow!("No resource address in state. Run `create` first."))?;

    // Ask the user which player is submitting the guess
    let round_guesses = state.current_round.as_ref().map(|r| &r.guesses);
    let user_names: Vec<String> = state
        .users
        .iter()
        .map(|u| {
            let guessed = round_guesses
                .and_then(|gs| gs.iter().find(|g| g.player_name == u.name))
                .map(|g| format!(" (guessed: {})", g.guess))
                .unwrap_or_default();
            format!("{}{}", u.name, guessed)
        })
        .collect();
    let user_idx = Select::new()
        .with_prompt("Select player submitting the guess")
        .default(0)
        .items(&user_names)
        .interact()?;
    let user = &state.users[user_idx];
    let user_name = user.name.clone();

    let number = if let Some(n) = number {
        if n > 10 {
            anyhow::bail!("Guess must be between 0 and 10");
        }
        n
    } else {
        Input::<u8>::new()
            .with_prompt("Enter your guess (0-10)")
            .validate_with(|input: &u8| -> Result<(), &str> {
                if *input <= 10 {
                    Ok(())
                } else {
                    Err("Guess must be between 0 and 10")
                }
            })
            .interact_text()?
    };

    let component_addr = state
        .component_address
        .ok_or_else(|| anyhow::anyhow!("No component address in state. Run `create` first."))?;

    let wallet = user.to_wallet(network);
    let indexer_url = state.indexer_url.clone();

    let mut provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(&indexer_url)
        .await?;

    let account_addr = provider.default_signer_address().to_account_address();

    let payout_component = user.account_address;

    let want_list = WantList::new()
        .add_vault_for_resource(account_addr, XTR, true)
        .add_vault_for_resource(component_addr, resource_address, true);

    println!(
        "🤔 Player '{}' is submitting guess {}...",
        user.name, number
    );
    build_and_send(
        &mut provider,
        |builder| {
            builder
                .pay_fee_from_component(account_addr, 2000u64)
                .call_method(component_addr, "guess", args![number, payout_component])
        },
        want_list,
    )
    .await?;

    println!("🤔 Guess {number} submitted successfully.");

    if let Some(round) = state.current_round.as_mut() {
        round.guesses.push(PlayerGuess {
            player_name: user_name,
            guess: number,
        });
        save_state(state_path, state)?;
    }

    Ok(())
}

// ────────────────────────────────── add-user ─────────────────────────────────
async fn cmd_add_user(state_path: &Path, state: &mut State) -> anyhow::Result<()> {
    create_user(state_path, state, None).await
}

async fn create_user(
    state_path: &Path,
    state: &mut State,
    name_arg: Option<String>,
) -> anyhow::Result<()> {
    let name = name_arg.unwrap_or_else(|| generate_name());
    let network = parse_network(&state.network)?;
    let secret = OotleSecretKey::random(network);
    let wallet = OotleWallet::from(PrivateKeyProvider::new(secret.clone()));

    let mut provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(&state.indexer_url)
        .await?;

    let address = secret.to_address();
    let account_address = address.to_account_address();

    println!("👤 Creating user '{name}' with account address {account_address}...");

    let unsigned_tx = IFaucet::new(&provider)
        .take_faucet_funds(10 * ONE_XTR)
        .pay_fee(500u64)
        .prepare()
        .await?;
    let tx = TransactionRequest::default()
        .with_transaction(unsigned_tx)
        .build(provider.wallet())
        .await?;

    let pending = provider.send_transaction(tx).await?;
    println!("⏳ Transaction submitted: {}", pending.tx_id());
    pending.watch().await?;
    println!("🚀 New user account created successfully.");

    let user = User {
        name,
        // WARNING: storing key material in state file is not secure! Don't do this. This is just for demo purposes.
        account_secret_hex: secret.account_secret().to_hex(),
        view_secret_hex: secret.view_only_secret().to_hex(),
        account_address,
    };

    println!("👤 Created user: {}", user.name);
    state.users.push(user);
    save_state(state_path, state)?;
    Ok(())
}

// ─────────────────────────────────── end-game ────────────────────────────────

async fn cmd_end_game(state: &mut State) -> anyhow::Result<()> {
    if !state.is_initialized() {
        anyhow::bail!("Wallet not initialized. Run `init` first.");
    }

    let component_addr = state
        .component_address
        .ok_or_else(|| anyhow::anyhow!("No component address in state. Run `create` first."))?;

    let wallet = wallet_from_state(state)?;
    let indexer_url = state.indexer_url.clone();
    let resource_address = state
        .resource_address
        .ok_or_else(|| anyhow::anyhow!("No resource address in state. Run `create` first."))?;

    let mut provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(&indexer_url)
        .await?;

    let account_addr = provider.default_signer_address().to_account_address();
    let want_list = WantList::new()
        .add_vault_for_resource(account_addr, XTR, true)
        .add_vault_for_resource(component_addr, resource_address, true);

    let all_player_accounts = state
        .users
        .iter()
        .filter(|u| {
            state
                .current_round
                .as_ref()
                .is_some_and(|r| r.guesses.iter().any(|g| g.player_name == u.name))
        })
        .map(|u| u.account_address);
    let prize = state
        .current_round
        .as_ref()
        .map(|r| NonFungibleAddress::from_str(&r.prize))
        .ok_or_else(|| anyhow::anyhow!("No active round found in state. Run `start-game` first."))?
        .expect("Invalid prize resource address in state");

    println!("🏆 Ending the round and paying out winner...");
    let receipt = build_and_send(
        &mut provider,
        |builder| {
            builder
                .add_input(prize)
                .with_inputs(all_player_accounts.map(Into::into))
                .pay_fee_from_component(account_addr, 2000u64)
                .call_method(component_addr, "end_game_and_payout", args![])
        },
        want_list,
    )
    .await?;

    let event = receipt
        .events
        .iter()
        .find(|e| e.topic() == "GuessingGame.GameEnded")
        .ok_or_else(|| anyhow::anyhow!("No GameEnded event found in receipt"))?;

    let winner = event.get_payload("winner_account");
    let number = event.get_payload("number");

    if let Some(winner) = winner {
        let winner = state
            .users
            .iter()
            .find(|u| u.account_address.to_string() == winner)
            .map(|u| format!("{} ({})", u.name, u.account_address))
            .unwrap_or_else(|| winner.to_string());
        println!("🏆 Winner: {} paid out", winner);
    } else {
        println!("🏆 No winner this round.");
    }
    println!(
        "🏆 The number was {}.",
        number.unwrap_or_else(|| "unknown?")
    );
    println!("🏆 Round ended.");

    Ok(())
}

// ─────────────────────────────────── show ────────────────────────────────────

async fn cmd_show(state: &State) -> anyhow::Result<()> {
    println!("🌐 Network:          {}", state.network);
    println!("🔗 Indexer:          {}", state.indexer_url);
    println!(
        "🏦 Account address:  {}",
        state
            .account_address
            .as_ref()
            .map(|a| a.to_string())
            .unwrap_or_else(|| "Not set".to_string())
    );
    println!(
        "📄 Template address: {}",
        state.template_address.as_deref().unwrap_or("Not set")
    );
    println!(
        "🎮 Game component:   {}",
        state.component_address.as_ref().display()
    );

    if !state.users.is_empty() {
        println!("\n👥 Players:");
        for user in &state.users {
            println!("  - {} ({})", user.name, user.account_address);
        }
    } else {
        println!("\n👥 Players: None");
    }

    match &state.current_round {
        None => {
            println!("\n🎲 Current round: None (run `start-game` to begin a round)");
        }
        Some(round) => {
            println!("\n🎲 Current round:");
            println!("  Prize: {}", round.prize);
            if round.guesses.is_empty() {
                println!("  Guesses: None yet");
            } else {
                println!("  Guesses ({}/{}):", round.guesses.len(), 5);
                for pg in &round.guesses {
                    println!("    - {}: {}", pg.player_name, pg.guess);
                }
            }
        }
    }

    Ok(())
}
