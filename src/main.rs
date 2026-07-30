pub mod crypto;
pub mod signaling;
pub mod webrtc;

use clap::{Parser, Subcommand};
use colored::*;
use inquire::{Select, Text};
use std::process::exit;

#[derive(Parser)]
#[command(name = "Nexus Remote")]
#[command(about = "Ultra-Low Latency Remote Desktop CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Host a remote desktop session
    Host {
        /// Optional PIN for the session. If not provided, one will be generated.
        #[arg(short, long)]
        pin: Option<String>,
        /// Signaling server URL
        #[arg(short, long, default_value = "ws://46.101.237.80:3000/ws")]
        server: String,
    },
    /// Connect to a remote desktop session
    Connect {
        /// PIN of the host to connect to
        #[arg(short, long)]
        pin: Option<String>,
        /// Signaling server URL
        #[arg(short, long, default_value = "ws://46.101.237.80:3000/ws")]
        server: String,
    },
}

#[tokio::main]
async fn main() {
    // Initialize GStreamer
    if let Err(err) = gstreamer::init() {
        eprintln!("{} Failed to initialize GStreamer: {}", "[-]".red().bold(), err);
        exit(1);
    }

    let cli = Cli::parse();

    println!("{}", "=========================================".cyan());
    println!("{} {}", "🚀 Nexus Remote".bold().cyan(), "v0.1.0");
    println!("{}", "Ultra-Low Latency Streaming & Control".bright_black());
    println!("{}", "=========================================".cyan());
    println!();

    match &cli.command {
        Some(Commands::Host { pin, server }) => {
            run_host(pin.clone(), server.clone()).await;
        }
        Some(Commands::Connect { pin, server }) => {
            run_connect(pin.clone(), server.clone()).await;
        }
        None => {
            interactive_mode().await;
        }
    }
}

async fn interactive_mode() {
    let options = vec!["Host a session (Share this PC)", "Connect to a session (Control another PC)"];
    
    let ans = Select::new("What would you like to do?", options).prompt();

    match ans {
        Ok("Host a session (Share this PC)") => {
            let server = Text::new("Signaling Server URL:")
                .with_default("ws://46.101.237.80:3000/ws")
                .prompt()
                .unwrap_or_else(|_| exit(0));
            run_host(None, server).await;
        }
        Ok("Connect to a session (Control another PC)") => {
            let server = Text::new("Signaling Server URL:")
                .with_default("ws://46.101.237.80:3000/ws")
                .prompt()
                .unwrap_or_else(|_| exit(0));
            run_connect(None, server).await;
        }
        _ => {
            println!("{}", "Exiting...".bright_black());
            exit(0);
        }
    }
}

async fn run_host(mut pin: Option<String>, server_url: String) {
    if pin.is_none() {
        let random_pin: u32 = (rand::random::<u32>() % 900_000) + 100_000;
        pin = Some(random_pin.to_string());
    }

    let pin = pin.unwrap();
    println!("{} Preparing to host session...", "[*]".blue().bold());
    println!("{} Your secure PIN is: {}", "[+]".green().bold(), pin.bold().yellow());
    
    // Connect to Signaling Server via WebSockets
    let sig_client = match signaling::SignalingClient::connect(&server_url, pin.clone()).await {
        Ok(client) => {
            println!("{} Connected to signaling server!", "[+]".green().bold());
            client
        }
        Err(e) => {
            eprintln!("{} Failed to connect to signaling server: {}", "[-]".red().bold(), e);
            eprintln!("{} Make sure the server is running and reachable", "[-]".red().bold());
            wait_for_enter();
            return;
        }
    };
        
    // Initialize GStreamer WebRTC pipeline for capturing screen
    println!("{} Starting WebRTC pipeline...", "[*]".blue().bold());
    match webrtc::start_host_pipeline(sig_client.rx, sig_client.tx).await {
        Ok(_) => {
            println!("{} Host pipeline started! Waiting for viewer...", "[+]".green().bold());
        }
        Err(e) => {
            eprintln!("{} Failed to start host pipeline: {}", "[-]".red().bold(), e);
            eprintln!("{} Make sure GStreamer is installed:", "[-]".red().bold());
            eprintln!("    macOS:   brew install gstreamer");
            eprintln!("    Linux:   sudo apt install gstreamer1.0-plugins-bad gstreamer1.0-plugins-good");
            eprintln!("    Windows: Install via MSI installer");
            wait_for_enter();
            return;
        }
    }
    
    // Block forever to keep the pipeline alive
    println!("{} Press Ctrl+C to stop hosting.", "[*]".blue().bold());
    tokio::signal::ctrl_c().await.unwrap();
    println!("{} Shutting down...", "[*]".blue().bold());
}

async fn run_connect(mut pin: Option<String>, server_url: String) {
    if pin.is_none() {
        let input = Text::new("Enter the 6-digit PIN of the host:").prompt();
        match input {
            Ok(p) => pin = Some(p),
            Err(_) => exit(0),
        }
    }

    let pin = pin.unwrap();
    println!("{} Connecting to host with PIN: {}...", "[*]".blue().bold(), pin.bold().yellow());
    
    // Connect to Signaling Server via WebSockets
    let sig_client = match signaling::SignalingClient::connect(&server_url, pin.clone()).await {
        Ok(client) => {
            println!("{} Connected to signaling server!", "[+]".green().bold());
            client
        }
        Err(e) => {
            eprintln!("{} Failed to connect to signaling server: {}", "[-]".red().bold(), e);
            eprintln!("{} Make sure the server is running and reachable", "[-]".red().bold());
            wait_for_enter();
            return;
        }
    };
        
    // Initialize GStreamer WebRTC pipeline for receiving and displaying video
    println!("{} Starting viewer pipeline...", "[*]".blue().bold());
    match webrtc::start_viewer_pipeline(sig_client.rx, sig_client.tx).await {
        Ok(_) => {
            println!("{} Viewer pipeline started! Receiving video...", "[+]".green().bold());
        }
        Err(e) => {
            eprintln!("{} Failed to start viewer pipeline: {}", "[-]".red().bold(), e);
            eprintln!("{} Make sure GStreamer is installed:", "[-]".red().bold());
            eprintln!("    macOS:   brew install gstreamer");
            eprintln!("    Linux:   sudo apt install gstreamer1.0-plugins-bad gstreamer1.0-plugins-good");
            eprintln!("    Windows: Install via MSI installer");
            wait_for_enter();
            return;
        }
    }
    
    // Block forever to keep the pipeline alive
    println!("{} Press Ctrl+C to disconnect.", "[*]".blue().bold());
    tokio::signal::ctrl_c().await.unwrap();
    println!("{} Shutting down...", "[*]".blue().bold());
}

fn wait_for_enter() {
    println!();
    println!("{} Press Enter to exit...", "[*]".blue().bold());
    let _ = std::io::stdin().read_line(&mut String::new());
}

