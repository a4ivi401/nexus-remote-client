pub mod config;
pub mod crypto;
pub mod protocol;
pub mod signaling;
pub mod webrtc;

use clap::{Parser, Subcommand};
use colored::*;
use config::AppConfig;
use inquire::{Confirm, Select, Text};
use protocol::{ClientMessage, ServerMessage};
use std::process::exit;
use tokio::sync::mpsc;
use tracing::{error, warn};

#[derive(Parser)]
#[command(name = "Nexus Remote")]
#[command(author = "Nexus Team")]
#[command(version = "0.2.0")]
#[command(about = "Ultra-Low Latency Cross-Platform Remote Desktop Client", long_about = None)]
struct Cli {
    /// Signaling Server URL (e.g. ws://127.0.0.1:3000/ws)
    #[arg(short, long, env = "NEXUS_SERVER")]
    server: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Host this desktop (share screen and accept remote control)
    Host {
        /// Signaling server URL
        #[arg(short, long)]
        server: Option<String>,

        /// Automatically accept incoming connection requests without prompt
        #[arg(short, long, default_value_t = false)]
        auto_accept: bool,
    },
    /// Connect to and view/control a remote host
    Connect {
        /// 6-digit Session ID of the remote host
        target_id: Option<String>,

        /// Signaling server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Configure default server URL and preferences
    Config {
        /// New default server URL to set
        #[arg(short, long)]
        server: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Initialize GStreamer runtime
    if let Err(err) = gstreamer::init() {
        eprintln!("{} Failed to initialize GStreamer: {}", "[-]".red().bold(), err);
        eprintln!("{} Please verify GStreamer runtime is properly installed.", "[-]".yellow());
        exit(1);
    }

    let cli = Cli::parse();
    let mut config = AppConfig::load();

    // Command line --server overrides config
    if let Some(srv) = cli.server {
        config.server_url = srv;
    }

    print_banner(&config.server_url);

    match cli.command {
        Some(Commands::Host { server, auto_accept }) => {
            let server_url = server.unwrap_or_else(|| config.server_url.clone());
            run_host(server_url, auto_accept).await;
        }
        Some(Commands::Connect { target_id, server }) => {
            let server_url = server.unwrap_or_else(|| config.server_url.clone());
            let target = match target_id {
                Some(id) => id,
                None => {
                    let default_target = config.last_target_id.clone().unwrap_or_default();
                    let prompt = Text::new("Enter remote 6-digit Session ID:");
                    let res = if !default_target.is_empty() {
                        prompt.with_default(&default_target).prompt()
                    } else {
                        prompt.prompt()
                    };
                    match res {
                        Ok(id) if !id.trim().is_empty() => id.trim().to_string(),
                        _ => exit(0),
                    }
                }
            };
            config.last_target_id = Some(target.clone());
            config.save();
            run_connect(server_url, target).await;
        }
        Some(Commands::Config { server }) => {
            if let Some(srv) = server {
                config.server_url = srv.clone();
                config.save();
                println!("{} Saved default server URL: {}", "[+]".green().bold(), srv.cyan());
            } else {
                let srv = Text::new("Signaling Server URL:")
                    .with_default(&config.server_url)
                    .prompt()
                    .unwrap_or_else(|_| exit(0));
                config.server_url = srv.clone();
                config.save();
                println!("{} Saved default server URL: {}", "[+]".green().bold(), srv.cyan());
            }
        }
        None => {
            interactive_menu(&mut config).await;
        }
    }
}

fn print_banner(server_url: &str) {
    println!("{}", "==========================================================".cyan());
    println!("{} {}", "🚀 Nexus Remote Client".bold().cyan(), "v0.2.0");
    println!("{}", "   High-Performance Open-Source Remote Desktop".bright_black());
    println!("   Active Server: {}", server_url.yellow());
    println!("{}", "==========================================================".cyan());
    println!();
}

async fn interactive_menu(config: &mut AppConfig) {
    loop {
        let options = vec![
            "📡 Host session (Share this PC)",
            "🖥️  Connect to remote PC (View & Control)",
            "⚙️  Settings (Change Signaling Server)",
            "❌ Exit",
        ];

        let selection = Select::new("Select an action:", options).prompt();

        match selection {
            Ok("📡 Host session (Share this PC)") => {
                run_host(config.server_url.clone(), config.auto_accept).await;
                break;
            }
            Ok("🖥️  Connect to remote PC (View & Control)") => {
                let default_target = config.last_target_id.clone().unwrap_or_default();
                let prompt = Text::new("Enter remote 6-digit Session ID:");
                let res = if !default_target.is_empty() {
                    prompt.with_default(&default_target).prompt()
                } else {
                    prompt.prompt()
                };

                match res {
                    Ok(id) if !id.trim().is_empty() => {
                        let target = id.trim().to_string();
                        config.last_target_id = Some(target.clone());
                        config.save();
                        run_connect(config.server_url.clone(), target).await;
                        break;
                    }
                    _ => continue,
                }
            }
            Ok("⚙️  Settings (Change Signaling Server)") => {
                let new_srv = Text::new("Signaling Server URL:")
                    .with_default(&config.server_url)
                    .prompt()
                    .unwrap_or_else(|_| config.server_url.clone());
                config.server_url = new_srv.clone();
                config.save();
                println!("{} Updated server URL: {}", "[+]".green().bold(), new_srv.cyan());
                println!();
            }
            _ => {
                println!("{}", "Goodbye!".bright_black());
                exit(0);
            }
        }
    }
}

async fn run_host(server_url: String, auto_accept: bool) {
    println!("{} Connecting to signaling server...", "[*]".blue().bold());

    let mut session = match signaling::SignalingClient::connect(&server_url).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} Failed to connect to signaling server: {}", "[-]".red().bold(), e);
            eprintln!("{} Server URL: {}", "[-]".red(), server_url);
            wait_for_enter();
            return;
        }
    };

    println!();
    println!("{}", "==========================================================".green());
    println!(
        "{} Your 6-digit Session ID: {}",
        "✨ Ready to Host! ".bold().green(),
        session.session_id.bold().yellow()
    );
    println!("   Share this ID with the person connecting to you.");
    println!("   Waiting for incoming connection requests...");
    println!("{}", "==========================================================".green());
    println!();

    let to_server_tx = session.to_server_tx.clone();
    let turn = session.turn.clone();
    let mut active_peer_tx: Option<mpsc::Sender<String>> = None;

    // Listen for server events
    while let Some(event) = session.server_events_rx.recv().await {
        match event {
            ServerMessage::ConnectionRequest { requester_id } => {
                println!();
                println!(
                    "{} Incoming connection request from ID: {}",
                    "🔔".bold(),
                    requester_id.bold().cyan()
                );

                let accepted = if auto_accept {
                    println!("{} Auto-accept enabled. Connection approved.", "[+]".green());
                    true
                } else {
                    let confirm = Confirm::new(&format!(
                        "Allow remote user [{}] to view your screen?",
                        requester_id
                    ))
                    .with_default(true)
                    .prompt();
                    confirm.unwrap_or(false)
                };

                let resp = ClientMessage::ConnectResponse {
                    requester_id: requester_id.clone(),
                    accepted,
                };
                let _ = to_server_tx.send(resp).await;

                if !accepted {
                    println!("{} Rejected connection request from {}.", "[-]".yellow(), requester_id);
                }
            }
            ServerMessage::ConnectionAccepted { peer_id } => {
                println!(
                    "{} Connection accepted with peer {}! Starting screen capture pipeline...",
                    "[+]".green().bold(),
                    peer_id.bold().yellow()
                );

                let (from_peer_tx, from_peer_rx) = mpsc::channel::<String>(100);
                active_peer_tx = Some(from_peer_tx);

                let to_srv = to_server_tx.clone();
                let turn_clone = turn.clone();
                tokio::spawn(async move {
                    if let Err(e) = webrtc::run_host_session(
                        peer_id,
                        turn_clone,
                        to_srv,
                        from_peer_rx,
                    )
                    .await
                    {
                        error!("WebRTC host session error: {}", e);
                    }
                });
            }
            ServerMessage::Signal { from_id: _, data } => {
                if let Some(ref tx) = active_peer_tx {
                    let _ = tx.send(data).await;
                } else {
                    warn!("Received signal from peer but no active session is running");
                }
            }
            ServerMessage::PeerDisconnected { peer_id } => {
                println!(
                    "{} Remote peer [{}] disconnected.",
                    "[-]".yellow().bold(),
                    peer_id
                );
                active_peer_tx = None;
                println!("{} Ready for new connections...", "[*]".blue());
            }
            ServerMessage::Error { code, message } => {
                eprintln!("{} Server error [{}]: {}", "[-]".red().bold(), code, message);
            }
            _ => {}
        }
    }

    println!("{} Host session terminated.", "[*]".bright_black());
}

async fn run_connect(server_url: String, target_id: String) {
    println!("{} Connecting to signaling server...", "[*]".blue().bold());

    let mut session = match signaling::SignalingClient::connect(&server_url).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} Failed to connect to signaling server: {}", "[-]".red().bold(), e);
            wait_for_enter();
            return;
        }
    };

    println!(
        "{} Requesting connection to remote Host [{}]...",
        "[*]".blue().bold(),
        target_id.bold().yellow()
    );

    // Send ConnectRequest
    let req = ClientMessage::ConnectRequest {
        target_id: target_id.clone(),
    };
    if let Err(e) = session.to_server_tx.send(req).await {
        eprintln!("{} Failed to send connect request: {}", "[-]".red().bold(), e);
        return;
    }

    let to_server_tx = session.to_server_tx.clone();
    let turn = session.turn.clone();
    let mut active_peer_tx: Option<mpsc::Sender<String>> = None;

    // Listen for server events
    while let Some(event) = session.server_events_rx.recv().await {
        match event {
            ServerMessage::ConnectionAccepted { peer_id } => {
                println!(
                    "{} Host [{}] accepted the connection! Initializing WebRTC viewer...",
                    "[+]".green().bold(),
                    peer_id.bold().green()
                );

                let (from_peer_tx, from_peer_rx) = mpsc::channel::<String>(100);
                active_peer_tx = Some(from_peer_tx);

                let to_srv = to_server_tx.clone();
                let turn_clone = turn.clone();
                tokio::spawn(async move {
                    if let Err(e) = webrtc::run_viewer_session(
                        peer_id,
                        turn_clone,
                        to_srv,
                        from_peer_rx,
                    )
                    .await
                    {
                        error!("WebRTC viewer session error: {}", e);
                    }
                });
            }
            ServerMessage::ConnectionRejected { peer_id, reason } => {
                println!(
                    "{} Host [{}] rejected or unavailable. Reason: {}",
                    "[-]".red().bold(),
                    peer_id,
                    reason.red()
                );
                wait_for_enter();
                return;
            }
            ServerMessage::Signal { from_id: _, data } => {
                if let Some(ref tx) = active_peer_tx {
                    let _ = tx.send(data).await;
                } else {
                    warn!("Received signal from peer but no active session is running");
                }
            }
            ServerMessage::PeerDisconnected { peer_id } => {
                println!(
                    "{} Remote host [{}] disconnected.",
                    "[-]".yellow().bold(),
                    peer_id
                );
                wait_for_enter();
                return;
            }
            ServerMessage::Error { code, message } => {
                eprintln!("{} Server error [{}]: {}", "[-]".red().bold(), code, message);
                wait_for_enter();
                return;
            }
            _ => {}
        }
    }
}

fn wait_for_enter() {
    println!();
    println!("{} Press Enter to exit...", "[*]".bright_black());
    let _ = std::io::stdin().read_line(&mut String::new());
}
