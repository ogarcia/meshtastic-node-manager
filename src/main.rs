//! Meshtastic Node Cleaner — remove stale or partially-known nodes from a
//! Meshtastic device's nodedb.

mod cli;
mod device;
mod display;
mod i18n;
mod nodes;

use std::io::{BufRead, Write};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use console::style;

use cli::Cli;
use device::{DEFAULT_TCP_PORT, Device, Endpoint, Target};
use nodes::{Selection, Skip, node_id};

fn main() -> ExitCode {
    let cli = cli::parse();

    let level = ["warn", "info", "debug"][usize::from(cli.verbose.min(2))];
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or(format!("warn,meshtastic_node_cleaner={level}")),
    )
    .format_timestamp(None)
    .init();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return fail(&e.into()),
    };
    match runtime.block_on(run(&cli)) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => fail(&e),
    }
}

fn fail(error: &anyhow::Error) -> ExitCode {
    eprintln!("{} {error:#}", style(fl!("error")).red().bold());
    ExitCode::FAILURE
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Connect, show the plan and remove the selected nodes.  Returns whether
/// every requested action succeeded.
async fn run(cli: &Cli) -> Result<bool> {
    display::banner();

    let endpoint = resolve_endpoint(&cli.target())?;
    let mut device = Device::connect(&endpoint)
        .await
        .map_err(|e| e.context(fl!("error-connect")))?;

    let result = clean(cli, &mut device).await;
    // Always release the serial port or socket, but report the first error.
    let closed = device.disconnect().await;
    let ok = result?;
    closed?;
    Ok(ok)
}

/// Turn the requested target into a concrete endpoint, auto-detecting the
/// serial port when needed.
fn resolve_endpoint(target: &Target) -> Result<Endpoint> {
    let endpoint = match target {
        Target::Tcp(host, port) => Endpoint::Tcp(host.clone(), *port),
        Target::Serial(Some(dev)) => Endpoint::Serial(dev.clone()),
        Target::Serial(None) | Target::Auto => {
            let ports = device::serial_ports()?;
            match ports.first() {
                Some(port) => {
                    if ports.len() > 1 {
                        println!(
                            "{}",
                            style(fl!(
                                "serial-multiple",
                                ports = ports.join(", "),
                                port = port.as_str()
                            ))
                            .yellow()
                        );
                    }
                    Endpoint::Serial(port.clone())
                }
                None if matches!(target, Target::Auto) => {
                    println!("{}", style(fl!("serial-none-fallback")).yellow());
                    Endpoint::Tcp("localhost".into(), DEFAULT_TCP_PORT)
                }
                None => return Err(anyhow!(fl!("error-no-serial"))),
            }
        }
    };
    let address = match &endpoint {
        Endpoint::Serial(dev) => fl!("connecting-serial", device = dev.as_str()),
        Endpoint::Tcp(host, port) => fl!("connecting-tcp", address = format!("{host}:{port}")),
    };
    println!("{}", style(address).cyan());
    Ok(endpoint)
}

async fn clean(cli: &Cli, device: &mut Device) -> Result<bool> {
    let mut ok = true;

    if cli.set_time {
        let now = u32::try_from(now()).unwrap_or(u32::MAX);
        match device.set_time(now).await {
            Ok(true) => println!("{}", style(fl!("time-set")).green()),
            Ok(false) => println!("{}", style(fl!("time-set-unconfirmed")).yellow()),
            Err(e) => {
                eprintln!("{} {e:#}", style(fl!("error-set-time")).red());
                ok = false;
            }
        }
    }

    let selection = cli.selection();
    let now = now();
    let mut plan = nodes::plan(&device.nodes, device.my_node_num, &selection, now);
    nodes::sort_nodes(&mut plan.nodes, cli.sort_by, cli.reverse);

    println!(
        "{}",
        style(fl!("nodedb-summary", count = device.nodes.len())).cyan()
    );
    println!();

    if plan.nodes.is_empty() && plan.skipped.is_empty() {
        display::panel(
            &style(fl!("nodedb-empty")).yellow().to_string(),
            console::Color::Yellow,
        );
        return Ok(ok);
    }

    let to_delete = plan.delete_count();
    println!("{}", style(selection_title(&selection, to_delete)).bold());
    if !plan.nodes.is_empty() {
        let own = matches!(selection, Selection::Filter { .. })
            .then_some(plan.own.as_ref())
            .flatten();
        display::nodes_table(own, &plan.nodes, now);
    } else {
        println!();
    }

    print_notes(&plan, &selection);

    if to_delete == 0 {
        display::panel(
            &format!(
                "{} {}",
                style(fl!("all-clean")).green().bold(),
                fl!("nothing-to-delete")
            ),
            console::Color::Green,
        );
        return Ok(ok);
    }

    if cli.dry_run {
        display::panel(
            &format!(
                "{} {}",
                style(fl!("dry-run")).yellow().bold(),
                fl!("dry-run-summary", count = to_delete)
            ),
            console::Color::Yellow,
        );
        return Ok(ok);
    }

    if !cli.yes && !confirm(&fl!("confirm", count = to_delete))? {
        println!("{}", style(fl!("cancelled")).yellow());
        return Ok(ok);
    }

    let mut removed = 0;
    for node in plan.to_delete() {
        let name = node.long_name.clone().unwrap_or_else(|| "—".into());
        match device.remove_node(node.num).await {
            Ok(true) => {
                removed += 1;
                println!(
                    "  {} {}",
                    style("✓").green(),
                    fl!("removed", id = node.id(), name = name)
                );
            }
            Ok(false) => {
                removed += 1;
                println!(
                    "  {} {}",
                    style("?").yellow(),
                    fl!("removed-unconfirmed", id = node.id(), name = name)
                );
            }
            Err(e) => {
                ok = false;
                println!(
                    "  {} {}",
                    style("✗").red(),
                    fl!("remove-failed", id = node.id(), error = format!("{e:#}"))
                );
            }
        }
    }
    println!();
    display::panel(
        &fl!("result", removed = removed, total = to_delete),
        if removed == to_delete {
            console::Color::Green
        } else {
            console::Color::Yellow
        },
    );
    Ok(ok)
}

fn selection_title(selection: &Selection, count: usize) -> String {
    let criteria = match selection {
        Selection::Explicit(_) => fl!("criteria-explicit"),
        Selection::Filter {
            unknown: true,
            older_than: None,
            ..
        } => fl!("criteria-unknown"),
        Selection::Filter {
            unknown,
            older_than: Some(age),
            include_never,
        } => {
            let age = nodes::human_delta(age.as_secs());
            let base = if *unknown {
                fl!("criteria-unknown-older", age = age)
            } else {
                fl!("criteria-older", age = age)
            };
            if *include_never {
                format!("{base} {}", fl!("criteria-include-never"))
            } else {
                base
            }
        }
        Selection::Filter { .. } => String::new(),
    };
    format!("{} — {criteria}", fl!("to-delete", count = count))
}

/// Explain nodes that were left out of the removal.
fn print_notes(plan: &nodes::Plan, selection: &Selection) {
    let mut printed = false;
    let mut note = |text: String| {
        println!("{}", style(text).yellow());
        printed = true;
    };

    for skip in &plan.skipped {
        note(match *skip {
            Skip::NotFound(num) => fl!("skip-not-found", id = node_id(num)),
            Skip::Own(num) => fl!("skip-own", id = node_id(num)),
            Skip::Favorite(num) => fl!("skip-favorite", id = node_id(num)),
            Skip::Ignored(num) => fl!("skip-ignored", id = node_id(num)),
        });
    }
    if matches!(selection, Selection::Filter { .. }) {
        let protected = plan.protected_count();
        if protected > 0 {
            note(fl!("note-protected", count = protected));
        }
    }
    let never = plan.never_heard_kept(selection);
    if never > 0 {
        note(fl!("note-never-heard", count = never));
    }
    if printed {
        println!();
    }
}

/// Ask a yes/no question on the terminal; anything but an explicit yes
/// (including end of input) is a no.
fn confirm(question: &str) -> Result<bool> {
    print!("{} {} ", style(question).bold(), fl!("confirm-hint"));
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    let answer = answer.trim().to_lowercase();
    Ok(!answer.is_empty()
        && fl!("confirm-yes-answers")
            .split(',')
            .any(|yes| yes.trim() == answer))
}
