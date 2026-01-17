//! Simple wget-like HTTP downloader using ureq (pure Rust, no async)

use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process;

fn print_usage() {
    eprintln!("Usage: wget [OPTIONS] URL");
    eprintln!("Options:");
    eprintln!("  -O FILE   Write output to FILE ('-' for stdout)");
    eprintln!("  -q        Quiet mode (no progress)");
    eprintln!("  -v        Verbose mode");
    eprintln!("  --help    Show this help");
}

fn download(url: &str, output: Option<&str>, quiet: bool, verbose: bool) -> Result<(), String> {
    if verbose {
        eprintln!("Connecting to {}...", url);
    }

    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    if status != 200 {
        return Err(format!("HTTP error: {} {}", status, response.status_text()));
    }

    let content_length: Option<usize> = response
        .header("content-length")
        .and_then(|s| s.parse().ok());

    if verbose {
        eprintln!("HTTP {} {}", status, response.status_text());
        if let Some(len) = content_length {
            eprintln!("Content-Length: {} bytes", len);
        }
    }

    // Determine output filename
    let output_path = match output {
        Some("-") => None, // stdout
        Some(path) => Some(path.to_string()),
        None => {
            // Extract filename from URL
            let path = url.rsplit('/').next().unwrap_or("index.html");
            let path = path.split('?').next().unwrap_or(path);
            if path.is_empty() {
                Some("index.html".to_string())
            } else {
                Some(path.to_string())
            }
        }
    };

    let mut reader = response.into_reader();
    let mut buffer = [0u8; 8192];
    let mut total_bytes = 0usize;

    if let Some(ref path) = output_path {
        // Write to file
        let mut file = File::create(path)
            .map_err(|e| format!("Cannot create '{}': {}", path, e))?;

        if !quiet {
            eprintln!("Saving to: '{}'", path);
        }

        loop {
            let bytes_read = reader.read(&mut buffer)
                .map_err(|e| format!("Read error: {}", e))?;
            if bytes_read == 0 {
                break;
            }
            file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Write error: {}", e))?;
            total_bytes += bytes_read;

            if !quiet {
                if let Some(len) = content_length {
                    let pct = (total_bytes * 100) / len;
                    eprint!("\r{} / {} bytes ({}%)", total_bytes, len, pct);
                } else {
                    eprint!("\r{} bytes", total_bytes);
                }
            }
        }

        file.sync_all().map_err(|e| format!("Sync error: {}", e))?;

        if !quiet {
            eprintln!("\nDownloaded {} bytes", total_bytes);
        }
    } else {
        // Write to stdout
        let mut stdout = io::stdout();
        loop {
            let bytes_read = reader.read(&mut buffer)
                .map_err(|e| format!("Read error: {}", e))?;
            if bytes_read == 0 {
                break;
            }
            stdout.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Write error: {}", e))?;
            total_bytes += bytes_read;
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut url: Option<String> = None;
    let mut output: Option<String> = None;
    let mut quiet = false;
    let mut verbose = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-O" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("wget: -O requires an argument");
                    process::exit(1);
                }
                output = Some(args[i].clone());
            }
            "-q" => quiet = true,
            "-v" => verbose = true,
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("wget: unknown option '{}'", arg);
                process::exit(1);
            }
            arg => {
                url = Some(arg.to_string());
            }
        }
        i += 1;
    }

    let url = match url {
        Some(u) => u,
        None => {
            print_usage();
            process::exit(1);
        }
    };

    if let Err(e) = download(&url, output.as_deref(), quiet, verbose) {
        eprintln!("wget: {}", e);
        process::exit(1);
    }
}
