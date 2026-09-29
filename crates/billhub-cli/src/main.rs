use billhub_core::{ImportFile, ImportOptions, LedgerStore, preview};
use chrono::DateTime;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "billhub",
    version,
    about = "Local bill import and ledger commands"
)]
struct Cli {
    #[arg(long, default_value = "billhub.sqlite3")]
    database: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Preview {
        file: PathBuf,
    },
    Import {
        file: PathBuf,
        #[arg(long)]
        replace: bool,
    },
    Batches,
    Events {
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long)]
        include_neutral: bool,
        #[arg(long)]
        include_pending: bool,
        #[arg(long)]
        json: bool,
    },
    Summary {
        #[arg(long)]
        include_neutral: bool,
        #[arg(long)]
        include_pending: bool,
    },
    Delete {
        batch_id: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let store = LedgerStore::open(&cli.database)?;
    match cli.command {
        Command::Preview { file: path } => {
            let file = ImportFile::from_path(&path)?;
            let result = preview(&store, &file)?;
            println!("{}", serde_json::to_string_pretty(&result.summary)?);
        }
        Command::Import { file, replace } => {
            let file = ImportFile::from_path(&file)?;
            let batch = billhub_core::import(
                &store,
                &file,
                ImportOptions {
                    replace,
                    source_path: Some(file.file_name.clone()),
                },
            )?;
            println!(
                "imported {} provider={} accepted={} rejected={} sha256={}",
                batch.id,
                batch.provider,
                batch.accepted_count,
                batch.rejected_count,
                batch.file_sha256
            );
        }
        Command::Batches => {
            for batch in store.batches()? {
                println!(
                    "{}\t{}\t{}\taccepted={}\trejected={}\tsha256={}",
                    batch.imported_at,
                    batch.id,
                    batch.provider,
                    batch.accepted_count,
                    batch.rejected_count,
                    batch.file_sha256
                );
            }
        }
        Command::Events {
            limit,
            include_neutral,
            include_pending,
            json,
        } => {
            let events = store.events(include_neutral, include_pending, limit)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&events)?);
                return Ok(());
            }
            for event in events {
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    DateTime::from_timestamp(event.occurred_at, 0)
                        .map(|v| v.to_rfc3339())
                        .unwrap_or_default(),
                    event.provider,
                    event.event_kind,
                    event.cash_flow,
                    event.amount_cents,
                    event.counterparty.unwrap_or_default(),
                    event.description.unwrap_or_default()
                );
            }
        }
        Command::Summary {
            include_neutral,
            include_pending,
        } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.summary(include_neutral, include_pending)?)?
            );
        }
        Command::Delete { batch_id } => {
            let deleted = store.delete_batch(&batch_id)?;
            if deleted {
                println!("deleted {batch_id}");
            } else {
                anyhow::bail!("batch not found: {batch_id}");
            }
        }
    }
    Ok(())
}
