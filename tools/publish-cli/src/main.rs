//! UltimaForge Publish CLI
//!
//! CLI tool for managing UltimaForge update manifests:
//! - Generate Ed25519 keypairs
//! - Create signed manifests from source directories
//! - Output content-addressed file blobs
//! - Validate update folder structure

mod keygen;
mod manifest;

use clap::{Parser, Subcommand};
use tracing::info;

/// UltimaForge Publish CLI - Manifest creation and signing tool
#[derive(Parser)]
#[command(name = "publish-cli")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new Ed25519 keypair for signing manifests
    Keygen {
        /// Output directory for key files
        #[arg(short, long, default_value = "./keys")]
        output: String,

        /// Force overwrite existing key files
        #[arg(short, long, default_value = "false")]
        force: bool,
    },

    /// Create a manifest from a source directory
    Manifest {
        /// Source directory containing files to include
        #[arg(short, long)]
        source: String,

        /// Output path for manifest.json
        #[arg(short, long, default_value = "./manifest.json")]
        output: String,

        /// Version string for the manifest
        #[arg(short, long, default_value = "1.0.0")]
        version: String,

        /// Client executable path (relative to source)
        #[arg(short, long, default_value = "client.exe")]
        executable: String,
    },

    /// Sign a manifest file with a private key
    Sign {
        /// Path to manifest.json
        #[arg(short, long)]
        manifest: String,

        /// Path to private key file
        #[arg(short, long)]
        key: String,

        /// Output path for signature file
        #[arg(short, long, default_value = "./manifest.sig")]
        output: String,
    },

    /// Copy files to content-addressed blob storage
    Blob {
        /// Source directory containing original files
        #[arg(short, long)]
        source: String,

        /// Output directory for content-addressed blobs
        #[arg(short, long, default_value = "./files")]
        output: String,
    },

    /// Validate an update folder structure
    Validate {
        /// Directory containing update files
        #[arg(short, long)]
        dir: String,

        /// Path to public key for signature verification
        #[arg(short, long)]
        key: String,
    },

    /// All-in-one publish workflow: manifest + sign + blob
    Publish {
        /// Source directory containing files to publish
        #[arg(short, long)]
        source: String,

        /// Output directory for update artifacts
        #[arg(short, long, default_value = "./updates")]
        output: String,

        /// Path to private key file
        #[arg(short, long)]
        key: String,

        /// Version string for the manifest
        #[arg(short, long, default_value = "1.0.0")]
        version: String,

        /// Client executable path (relative to source)
        #[arg(short, long, default_value = "client.exe")]
        executable: String,
    },
}

fn main() {
    // Initialize tracing for structured logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Keygen { output, force } => {
            info!("Generating keypair to: {}", output);
            match keygen::generate_keypair(&output, force) {
                Ok(result) => {
                    println!("✓ Generated Ed25519 keypair successfully!");
                    println!();
                    println!("Files created:");
                    println!("  Private key: {}", result.private_key_path);
                    println!("  Public key:  {}", result.public_key_path);
                    println!();
                    println!("Public key (for embedding in launcher):");
                    println!("  {}", result.public_key_hex);
                    println!();
                    println!("⚠ SECURITY: Keep private.key secure and never distribute it!");
                }
                Err(e) => {
                    eprintln!("✗ Failed to generate keypair: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Manifest {
            source,
            output,
            version,
            executable,
        } => {
            info!(
                "Creating manifest from: {} -> {}",
                source, output
            );
            match manifest::generate_manifest(&source, &output, &version, &executable) {
                Ok(result) => {
                    println!("✓ Generated manifest successfully!");
                    println!();
                    println!("Manifest: {}", result.manifest_path);
                    println!("  Version: {}", result.version);
                    println!("  Files:   {}", result.file_count);
                    println!("  Size:    {}", manifest::format_size(result.total_size));
                }
                Err(e) => {
                    eprintln!("✗ Failed to generate manifest: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Sign {
            manifest,
            key,
            output,
        } => {
            info!("Signing manifest: {} with key: {}", manifest, key);
            // TODO: Implement signing in subtask-5-3
            println!(
                "Sign command placeholder - manifest: {}, key: {}, output: {}",
                manifest, key, output
            );
        }
        Commands::Blob { source, output } => {
            info!("Creating blobs from: {} -> {}", source, output);
            // TODO: Implement blob creation in subtask-5-4
            println!(
                "Blob command placeholder - source: {}, output: {}",
                source, output
            );
        }
        Commands::Validate { dir, key } => {
            info!("Validating update folder: {} with key: {}", dir, key);
            // TODO: Implement validation in subtask-5-5
            println!(
                "Validate command placeholder - dir: {}, key: {}",
                dir, key
            );
        }
        Commands::Publish {
            source,
            output,
            key,
            version,
            executable,
        } => {
            info!(
                "Publishing from: {} -> {} (v{})",
                source, output, version
            );
            // TODO: Implement full publish workflow in subtask-5-6
            println!(
                "Publish command placeholder - source: {}, output: {}, key: {}, version: {}, executable: {}",
                source, output, key, version, executable
            );
        }
    }
}
