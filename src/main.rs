use clap::Parser as _;
use color_eyre::eyre::Context as _;
use tracing::debug;

#[derive(Debug, clap::Parser)]
#[clap(about, version)]
struct Args {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,
}

#[test]
fn args() {
    <Args as clap::CommandFactory>::command().debug_assert();
}

fn main() -> color_eyre::Result<()> {
    let args = Args::parse();
    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive({
            use tracing_subscriber::filter::LevelFilter;
            match args.verbose.log_level() {
                Some(log::Level::Error) => LevelFilter::ERROR,
                Some(log::Level::Warn) => LevelFilter::WARN,
                Some(log::Level::Info) => LevelFilter::INFO,
                Some(log::Level::Debug) => LevelFilter::DEBUG,
                Some(log::Level::Trace) => LevelFilter::TRACE,
                None => LevelFilter::OFF,
            }
            .into()
        })
        .from_env()
        .context("couldn't parse RUST_LOG environment variable")?;
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    debug!(?args);
    Ok(())
}
