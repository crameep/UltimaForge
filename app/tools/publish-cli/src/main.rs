//! UltimaForge Publish CLI
//!
//! CLI tool for managing UltimaForge update manifests:
//! - Generate Ed25519 keypairs
//! - Create signed manifests from source directories
//! - Output content-addressed file blobs
//! - Validate update folder structure

mod blob;
mod keygen;
mod manifest;
mod sign;
mod validate;

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
            match sign::sign_manifest(&manifest, &key, &output) {
                Ok(result) => {
                    println!("✓ Signed manifest successfully!");
                    println!();
                    println!("Signature: {}", result.signature_path);
                    println!("  Manifest size: {} bytes", result.manifest_size);
                    println!();
                    println!("Signature (hex):");
                    println!("  {}", result.signature_hex);
                }
                Err(e) => {
                    eprintln!("✗ Failed to sign manifest: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Blob { source, output } => {
            info!("Creating blobs from: {} -> {}", source, output);
            match blob::create_blobs(&source, &output) {
                Ok(result) => {
                    println!("✓ Created content-addressed blobs successfully!");
                    println!();
                    println!("Output: {}", result.output_dir);
                    println!("  Unique blobs:  {}", result.blob_count);
                    println!("  Total files:   {}", result.blobs.len());
                    println!("  Deduplicated:  {}", result.deduplicated_count);
                    println!("  Total size:    {}", blob::format_size(result.total_size));
                }
                Err(e) => {
                    eprintln!("✗ Failed to create blobs: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Validate { dir, key } => {
            info!("Validating update folder: {} with key: {}", dir, key);
            match validate::validate_update_folder(&dir, &key) {
                Ok(result) => {
                    println!("✓ Validation completed successfully!");
                    println!();
                    println!("Update folder: {}", result.dir_path);
                    println!("  Version:     {}", result.version);
                    println!("  Signature:   {}", if result.signature_valid { "Valid" } else { "Invalid" });
                    println!("  Files:       {}", result.file_count);
                    println!("  Verified:    {}", result.files_verified);
                    println!("  Missing:     {}", result.missing_blobs);
                    println!("  Total size:  {}", validate::format_size(result.total_size));

                    if !result.missing_blob_paths.is_empty() {
                        println!();
                        println!("⚠ Missing blobs:");
                        for path in &result.missing_blob_paths {
                            println!("  - {}", path);
                        }
                    }

                    if result.missing_blobs > 0 {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("✗ Validation failed: {}", e);
                    std::process::exit(1);
                }
            }
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

            // Compute output paths
            let manifest_path = format!("{}/manifest.json", output);
            let signature_path = format!("{}/manifest.sig", output);
            let blobs_path = format!("{}/files", output);

            println!("Publishing v{} from: {}", version, source);
            println!();

            // Step 1: Generate manifest
            println!("Step 1/3: Generating manifest...");
            let manifest_result = match manifest::generate_manifest(&source, &manifest_path, &version, &executable) {
                Ok(result) => {
                    println!("  ✓ Generated manifest with {} files ({})",
                        result.file_count,
                        manifest::format_size(result.total_size)
                    );
                    result
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to generate manifest: {}", e);
                    std::process::exit(1);
                }
            };

            // Step 2: Sign manifest
            println!("Step 2/3: Signing manifest...");
            match sign::sign_manifest(&manifest_path, &key, &signature_path) {
                Ok(result) => {
                    println!("  ✓ Signed manifest ({} bytes)", result.manifest_size);
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to sign manifest: {}", e);
                    std::process::exit(1);
                }
            }

            // Step 3: Create content-addressed blobs
            println!("Step 3/3: Creating content-addressed blobs...");
            let blob_result = match blob::create_blobs(&source, &blobs_path) {
                Ok(result) => {
                    println!("  ✓ Created {} unique blobs ({})",
                        result.blob_count,
                        blob::format_size(result.total_size)
                    );
                    if result.deduplicated_count > 0 {
                        println!("    ({} files deduplicated)", result.deduplicated_count);
                    }
                    result
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to create blobs: {}", e);
                    std::process::exit(1);
                }
            };

            // Summary
            println!();
            println!("✓ Published successfully!");
            println!();
            println!("Output directory: {}", output);
            println!("  manifest.json  - {} files, {}", manifest_result.file_count, manifest::format_size(manifest_result.total_size));
            println!("  manifest.sig   - Ed25519 signature");
            println!("  files/         - {} content-addressed blobs", blob_result.blob_count);
            println!();
            println!("Version: {}", version);
            println!("Executable: {}", executable);
        }
    }
}
