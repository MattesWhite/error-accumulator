//! This example CLI tool compares the usage of
//! [`ErrorAccumulator`](error_accumulator::ErrorAccumulator) with a common
//! approach of using the `?` operator on each conversion for early returns.
//!
//! By default the early return method is used. As a result, only the first
//! error in the accompanying `config.yaml` is reported.
//!
//! Run the CLI with the `--accumulate` flag to see all errors of the config
//! accumulated.

use std::{fs::File, path::PathBuf, thread::sleep, time::Duration};

use clap::Parser;
use reqwest::{StatusCode, Url};
use serde::Deserialize;

/// Simple request tool that sends HTTP GET requests to all configured hosts in
/// a configured interval and print the results.
#[derive(Debug, Parser)]
struct Cli {
    /// Accumulate errors.
    #[arg(short, long)]
    accumulate: bool,
    /// Path to the config file.
    #[arg(short, long, default_value = "./config.yaml")]
    config: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RawHost {
    url: String,
    expected_status: u16,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    interval: String,
    hosts: Vec<RawHost>,
}

#[derive(Debug)]
struct Host {
    url: Url,
    expected_status: StatusCode,
}

#[derive(Debug)]
struct Config {
    interval: Duration,
    hosts: Vec<Host>,
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    let file = File::open(cli.config)?;
    let raw_config = serde_yml::from_reader::<_, RawConfig>(file)?;

    let config = if cli.accumulate {
        accumulate::parse(raw_config)?
    } else {
        no_acc::parse(raw_config)?
    };

    loop {
        for host in &config.hosts {
            let resp = reqwest::blocking::get(host.url.clone())?;
            eprintln!(
                "Requested '{}': got expected status code ({}): {}",
                host.url,
                host.expected_status,
                host.expected_status == resp.status()
            );
        }
        sleep(config.interval);
    }
}

mod no_acc {
    use eyre::{Context, eyre};
    use reqwest::StatusCode;

    use crate::{Config, Host, RawConfig};

    pub fn parse(raw: RawConfig) -> eyre::Result<Config> {
        let interval = raw.interval.parse::<humantime::Duration>()?;
        let hosts = raw
            .hosts
            .into_iter()
            .map(|host| {
                StatusCode::from_u16(host.expected_status)
                    .wrap_err(eyre!("invalid StatusCode"))
                    .and_then(|sc| {
                        host.url
                            .parse()
                            .wrap_err(eyre!("invalid URL"))
                            .map(|url| Host {
                                url,
                                expected_status: sc,
                            })
                    })
            })
            .collect::<Result<Vec<_>, eyre::Report>>()?;

        Ok(Config {
            interval: interval.into(),
            hosts,
        })
    }
}

mod accumulate {
    use error_accumulator::{ErrorAccumulator, error::AccumulatedError, path::FieldName};
    use reqwest::{StatusCode, Url};

    use crate::{Config, Host, RawConfig};

    const INTERVAL: FieldName = FieldName::new_unchecked("interval");
    const HOSTS: FieldName = FieldName::new_unchecked("hosts");
    const URL: FieldName = FieldName::new_unchecked("url");
    const EXPECTED_STATUS: FieldName = FieldName::new_unchecked("expected_status");

    pub fn parse(raw: RawConfig) -> Result<Config, AccumulatedError> {
        ErrorAccumulator::new()
            .field(
                INTERVAL,
                raw.interval.parse::<humantime::Duration>().map(Into::into),
            )
            .array(HOSTS)
            .of_structs(raw.hosts, |strukt, host| {
                strukt
                    .field_builder(URL)
                    .value(host.url.parse::<Url>())
                    .with_previous(|url: &Url| reqwest::blocking::get(url.clone()))
                    .on_ok(|url, _| url)
                    .finish()
                    .field(EXPECTED_STATUS, StatusCode::from_u16(host.expected_status))
                    .on_ok(|url, expected_status| Host {
                        url,
                        expected_status,
                    })
                    .finish()
            })
            .finish()
            .on_ok(|interval, hosts| Config { interval, hosts })
            .analyse()
    }
}
