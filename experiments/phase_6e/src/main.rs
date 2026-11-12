//! Phase 6E Binary CLI runner.

use blindmark_phase_6e::config::ServiceConfig;
use blindmark_phase_6e::helper_service::HelperService;
use blindmark_phase_6e::leader_service::LeaderService;
use blindmark_phase_6c::protocol::SEED_SIZE;
use rand::Rng;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let role = args.get(1).map(|s| s.as_str()).unwrap_or("leader");

    println!("=== Blindmark Phase 6E: Rust Aggregator HTTP Service ===");

    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();

    match role {
        "leader" => {
            let config = ServiceConfig::default_leader();
            println!("Starting Leader Service on {}:{}", config.host, config.port);
            let service = LeaderService::new(config, verify_key).expect("Failed to create LeaderService");
            let _handle = service.spawn().expect("Failed to spawn listener");
            println!("Leader Service listening on port 8001. Press Ctrl+C to terminate.");
            std::thread::park();
        }
        "helper" => {
            let config = ServiceConfig::default_helper();
            println!("Starting Helper Service on {}:{}", config.host, config.port);
            let service = HelperService::new(config, verify_key).expect("Failed to create HelperService");
            let _handle = service.spawn().expect("Failed to spawn listener");
            println!("Helper Service listening on port 8002. Press Ctrl+C to terminate.");
            std::thread::park();
        }
        _ => {
            eprintln!("Usage: blindmark-phase-6e [leader|helper]");
            std::process::exit(1);
        }
    }
}

