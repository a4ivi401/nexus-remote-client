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
    },
    /// Connect to a remote desktop session
    Connect {
        /// PIN of the host to connect to
        #[arg(short, long)]
        pin: Option<String>,
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
        Some(Commands::Host { pin }) => {
            run_host(pin.clone()).await;
        }
        Some(Commands::Connect { pin }) => {
            run_connect(pin.clone()).await;
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
            run_host(None).await;
        }
        Ok("Connect to a session (Control another PC)") => {
            run_connect(None).await;
        }
        _ => {
            println!("{}", "Exiting...".bright_black());
            exit(0);
        }
    }
}

async fn run_host(mut pin: Option<String>) {
    if pin.is_none() {
        use rand::Rng;
        let random_pin: u32 = (rand::random::<u32>() % 900_000) + 100_000;
        pin = Some(random_pin.to_string());
    }

    let pin = pin.unwrap();
    println!("{} Preparing to host session...", "[*]".blue().bold());
    println!("{} Your secure PIN is: {}", "[+]".green().bold(), pin.bold().yellow());
    println!("{} Waiting for viewer to connect...", "[*]".blue().bold());
    
    // Connect to Signaling Server via WebSockets
    let sig_client = signaling::SignalingClient::connect("ws://127.0.0.1:3000/ws", pin.clone())
        .await
        .expect("Failed to connect to signaling server");
        
    // Initialize GStreamer WebRTC pipeline for capturing screen
    webrtc::start_host_pipeline(sig_client.rx, sig_client.tx)
        .await
        .expect("Failed to start host pipeline");
    
    // Block forever to keep the pipeline alive
    tokio::signal::ctrl_c().await.unwrap();
    println!("{} Shutting down...", "[*]".blue().bold());
}

async fn run_connect(mut pin: Option<String>) {
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
    let sig_client = signaling::SignalingClient::connect("ws://127.0.0.1:3000/ws", pin.clone())
        .await
        .expect("Failed to connect to signaling server");
        
    // Initialize GStreamer WebRTC pipeline for receiving and displaying video
    webrtc::start_viewer_pipeline(sig_client.rx, sig_client.tx)
        .await
        .expect("Failed to start viewer pipeline");
    
    // Block forever to keep the pipeline alive
    tokio::signal::ctrl_c().await.unwrap();
    println!("{} Shutting down...", "[*]".blue().bold());
}
