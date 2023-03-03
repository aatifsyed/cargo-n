use clap::Parser as _;
use color_eyre::eyre::{bail, Context as _};
use itertools::Itertools as _;
use std::{
    ffi::{OsStr, OsString},
    fmt,
    path::{Path, PathBuf},
};
use tap::{Pipe as _, Tap as _};
use tracing::{debug, info};

#[derive(Debug, clap::Parser)]
#[clap(about, version)]
struct Args {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,
    /// Use a binary (application) template [default]
    #[arg(long)]
    bin: bool,
    /// Use a library template
    #[arg(long, conflicts_with = "bin")]
    lib: bool,
    /// Set the resulting package name, defaults to the directory name
    #[arg(long)]
    name: Option<String>,
    path: PathBuf,
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

    let Args {
        verbose: _,
        bin,
        lib,
        name,
        path,
    } = args;

    let mut cargo_args = vec![OsString::from("new")];

    if bin {
        cargo_args.push("--bin".into())
    }
    if lib {
        cargo_args.push("--lib".into())
    }
    if let Some(name) = name {
        cargo_args.push("--name".into());
        cargo_args.push(name.into())
    }
    cargo_args.push(path.clone().into());

    run_cmd("cargo", &cargo_args, None)?;
    run_cmd(
        "git",
        [
            "commit",
            "--allow-empty",
            "--message",
            "bootstrap: root commit",
        ],
        path.as_path(),
    )?;
    run_cmd("git", ["add", "."], path.as_path())?;
    run_cmd(
        "git",
        [
            OsString::from("commit"),
            OsString::from("--message"),
            os_string_join(cargo_args.tap_mut(|it| it.insert(0, "bootstrap: cargo".into()))),
        ],
        path.as_path(),
    )?;

    if !lib {
        run_cmd("cargo", ["build"], path.as_path())?;
        run_cmd("git", ["add", "."], path.as_path())?;
        run_cmd(
            "git",
            ["commit", "--message", "bootstrap: initial build"],
            path.as_path(),
        )?;
    }

    Ok(())
}

fn run_cmd<'a, ArgT>(
    cmd: &str,
    args: impl IntoIterator<Item = ArgT>,
    dir: impl Into<Option<&'a Path>>,
) -> color_eyre::Result<()>
where
    ArgT: fmt::Debug + AsRef<OsStr>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    let dir = dir.into();
    use std::process::{Command, Stdio};
    match args.is_empty() {
        true => info!("running {cmd}"),
        false => info!("running {cmd} with args {args:?} with directory {dir:?}"),
    }
    match Command::new(cmd)
        .args(&args)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .pipe(|it| match &dir {
            Some(dir) => it.current_dir(dir),
            None => it,
        })
        .status()
    {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => {
            bail!("command {cmd} with args {args:?} in dir {dir:?} failed with status {status:?}")
        }
        Err(err) => bail!("failed to run command {cmd} in dir {dir:?}: {err}"),
    }
}

fn os_string_join<T>(strings: impl IntoIterator<Item = T>) -> OsString
where
    T: AsRef<OsStr>,
{
    let mut joined = OsString::new();
    for s in strings.into_iter().with_position() {
        use itertools::Position::{First, Last, Middle, Only};
        match s {
            First(s) | Middle(s) => {
                joined.push(s);
                joined.push(" ")
            }
            Last(s) | Only(s) => joined.push(s),
        }
    }
    joined
}
