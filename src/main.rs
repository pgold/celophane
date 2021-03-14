use anyhow::{anyhow, Result};
use ethers::providers::{Http, Middleware, Provider, Ws};
use ethers::types::{Address, U256};
use std::convert::TryFrom;
use std::future::Future;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::join;
use url::Url;

mod celo;

#[derive(StructOpt)]
struct CelophaneOpt {
    /// Endpoint to connect to.
    #[structopt(long, default_value = "http://localhost:8545")]
    endpoint: Url,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
struct AccountBalanceOpt {
    address: Address,
}

#[derive(StructOpt)]
enum AccountCommand {
    /// Retrieve an account's balance.
    Balance(AccountBalanceOpt),
}

#[derive(StructOpt)]
struct ExchangeShowOpt {
    /// Amount (base) to report quote amounts on.
    #[structopt(long, default_value = "1000000000000000000", parse(try_from_str=U256::from_dec_str))]
    amount: U256,
}

#[derive(StructOpt)]
enum ExchangeCommand {
    /// Display the on-chain exchange (Mento) rates.
    Show(ExchangeShowOpt),
}

#[derive(StructOpt)]
enum Command {
    /// Account management.
    Account(AccountCommand),
    /// On-chain exchange (Mento) interaction.
    Exchange(ExchangeCommand),
}

fn print_balance(maybe_balance: Option<U256>, label: &str) {
    match maybe_balance {
        Some(balance) => println!("{}: {}", label, balance),
        None => (),
    }
}

async fn get_balance<M: Middleware, Fut>(token_future: Fut, address: Address) -> Option<U256>
where
    Fut: Future<Output = Result<celo::Erc20<M>>>,
{
    // One would expect a non-existent token to be an Error on maybe_token,
    // but there is a token and the balance_of call fails instead.
    match token_future.await {
        Ok(token) => match token.balance_of(address).call().await {
            Ok(balance) => Some(balance),
            // Ignore unexistent token.
            Err(_) => None,
        },
        // Ignore unexistent token.
        Err(_) => None,
    }
}

async fn account_balance<M: Middleware>(client: Arc<M>, args: AccountBalanceOpt) -> Result<()> {
    let (celo_balance, cusd_balance, ceur_balance) = join!(
        get_balance(celo::get_celo_token(client.clone()), args.address),
        get_balance(celo::get_cusd_token(client.clone()), args.address),
        get_balance(celo::get_ceur_token(client.clone()), args.address)
    );

    println!("All balances expressed in units of 10^-18.");
    print_balance(celo_balance, "CELO");
    print_balance(cusd_balance, "cUSD");
    print_balance(ceur_balance, "cEUR");

    Ok(())
}

async fn exchange_show<M: Middleware>(client: Arc<M>, args: ExchangeShowOpt) -> Result<()> {
    let exchange = celo::get_exchange(client.clone()).await?;

    let base_qty = args.amount;

    let cusd_call = exchange.get_buy_token_amount(base_qty, true);
    let celo_call = exchange.get_buy_token_amount(base_qty, false);

    let (cusd_quote_qty, celo_quote_qty) = join!(cusd_call.call(), celo_call.call());

    println!("{} CELO => {} cUSD", base_qty, cusd_quote_qty.unwrap());
    println!("{} cUSD => {} CELO", base_qty, celo_quote_qty.unwrap());

    Ok(())
}

async fn run_command<M: Middleware>(provider: M, args: CelophaneOpt) -> Result<()> {
    let client = Arc::new(provider);
    match args.cmd {
        Command::Account(opt) => match opt {
            AccountCommand::Balance(opt) => account_balance(client, opt).await?,
        },
        Command::Exchange(opt) => match opt {
            ExchangeCommand::Show(opt) => exchange_show(client, opt).await?,
        },
    }

    Ok(())
}

enum EitherProvider {
    Http(Provider<Http>),
    Ws(Provider<Ws>),
}

async fn create_provider(url: &Url) -> Result<EitherProvider> {
    let provider = match url.scheme() {
        "https" | "http" => EitherProvider::Http(Provider::<Http>::try_from(url.as_str())?),
        "wss" | "ws" => EitherProvider::Ws(Provider::<Ws>::connect(url.as_str()).await?),
        scheme => Err(anyhow!("Unknown URL scheme \"{}\"", scheme))?,
    };
    Ok(provider)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CelophaneOpt::from_args();

    let provider = create_provider(&args.endpoint).await?;
    let result = match provider {
        EitherProvider::Http(p) => run_command(p, args).await,
        EitherProvider::Ws(p) => run_command(p, args).await,
    };

    result
}
