// src-tauri/src/lib.rs
mod virtual_adapter;
mod packet_router;
mod performance_monitor;
pub mod interface_manager;

use std::sync::Arc;
use tokio::sync::RwLock;
use virtual_adapter::VirtualNetworkInterface;
use packet_router::LoadBalancingMode;
use performance_monitor::{PerformanceStats};
use interface_manager::{InterfaceManager, PhysicalInterface};

// Global state for the application
pub struct AppState {
    pub virtual_interface: Arc<RwLock<Option<VirtualNetworkInterface>>>,
    pub is_running: Arc<RwLock<bool>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            virtual_interface: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[cfg(feature = "gui")]
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to NetBoost Pro!", name)
}

#[cfg(feature = "gui")]
#[tauri::command]
async fn start_netboost(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let is_running = *state.is_running.read().await;
    
    if is_running {
        return Err("NetBoost Pro is already running".to_string());
    }

    println!("Starting NetBoost Pro service...");
    
    match VirtualNetworkInterface::new().await {
        Ok(vni) => {
            *state.virtual_interface.write().await = Some(vni);
            *state.is_running.write().await = true;
            
            // Start the virtual interface in a background task
            let vni_state = Arc::clone(&state.virtual_interface);
            let running_state = Arc::clone(&state.is_running);
            
            tauri::async_runtime::spawn(async move {
                if let Some(vni) = vni_state.write().await.take() {
                    if let Err(e) = vni.run().await {
                        eprintln!("Virtual interface error: {}", e);
                    }
                }
                *running_state.write().await = false;
            });
            
            Ok("NetBoost Pro started successfully".to_string())
        }
        Err(e) => {
            eprintln!("Failed to start NetBoost Pro: {}", e);
            Err(format!("Failed to start NetBoost Pro: {}", e))
        }
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
async fn stop_netboost(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let is_running = *state.is_running.read().await;
    
    if !is_running {
        return Err("NetBoost Pro is not running".to_string());
    }

    println!("Stopping NetBoost Pro service...");
    
    if let Some(vni) = state.virtual_interface.read().await.as_ref() {
        vni.stop().await;
    }
    
    *state.is_running.write().await = false;
    *state.virtual_interface.write().await = None;
    
    Ok("NetBoost Pro stopped successfully".to_string())
}

#[cfg(feature = "gui")]
#[tauri::command]
async fn get_service_status(state: tauri::State<'_, AppState>) -> Result<ServiceStatus, String> {
    let is_running = *state.is_running.read().await;
    
    Ok(ServiceStatus {
        is_running,
        uptime_seconds: if is_running { Some(3600) } else { None }, // Placeholder
        virtual_interface_name: if is_running { 
            Some("NetBoost-TUN".to_string()) 
        } else { 
            None 
        },
    })
}

#[cfg(feature = "gui")]
#[tauri::command]
async fn get_performance_stats(state: tauri::State<'_, AppState>) -> Result<PerformanceStats, String> {
    let is_running = *state.is_running.read().await;
    
    if !is_running {
        return Err("NetBoost Pro is not running".to_string());
    }

    if let Some(vni) = state.virtual_interface.read().await.as_ref() {
        Ok(vni.get_performance_stats().await)
    } else {
        Err("Virtual interface not available".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
async fn get_network_interfaces() -> Result<Vec<PhysicalInterface>, String> {
    match InterfaceManager::new() {
        Ok(manager) => {
            // Return all discovered interfaces
            Ok(vec![manager.get_primary_interface().unwrap().clone()])
        }
        Err(e) => Err(format!("Failed to discover interfaces: {}", e)),
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
async fn set_load_balancing_mode(
    mode: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let is_running = *state.is_running.read().await;
    
    if !is_running {
        return Err("NetBoost Pro is not running".to_string());
    }

    let balancing_mode = match mode.as_str() {
        "round_robin" => LoadBalancingMode::RoundRobin,
        "latency_based" => LoadBalancingMode::LatencyBased,
        "bandwidth_based" => LoadBalancingMode::BandwidthBased,
        "balanced" => LoadBalancingMode::Balanced,
        _ => return Err("Invalid load balancing mode".to_string()),
    };

    if let Some(vni) = state.virtual_interface.write().await.as_mut() {
        vni.set_load_balancing_mode(balancing_mode).await;
        Ok(format!("Load balancing mode set to: {}", mode))
    } else {
        Err("Virtual interface not available".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
async fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_date: env!("BUILD_DATE").unwrap_or("unknown").to_string(),
    })
}

// Data structures for Tauri commands
#[cfg(feature = "gui")]
#[derive(serde::Serialize, serde::Deserialize)]
struct ServiceStatus {
    is_running: bool,
    uptime_seconds: Option<u64>,
    virtual_interface_name: Option<String>,
}

#[cfg(feature = "gui")]
#[derive(serde::Serialize, serde::Deserialize)]
struct SystemInfo {
    os: String,
    arch: String,
    version: String,
    build_date: String,
}

#[cfg(feature = "gui")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    #[cfg(debug_assertions)]
    {
        env_logger::init();
    }

    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            // You can perform additional setup here if needed
            println!("NetBoost Pro GUI initialized");
            
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            start_netboost,
            stop_netboost,
            get_service_status,
            get_performance_stats,
            get_network_interfaces,
            set_load_balancing_mode,
            get_system_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(not(feature = "gui"))]
pub fn run() {
    eprintln!("GUI feature not enabled. Use the CLI instead.");
    std::process::exit(1);
}