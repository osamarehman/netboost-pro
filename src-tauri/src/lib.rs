mod virtual_adapter;
pub mod interface_manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[cfg(feature = "gui")]
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg(feature = "gui")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
        tauri::Builder::default()
        .setup(|_app| {
            // Initialize the virtual network interface in a background task
            tauri::async_runtime::spawn(async move {
                match virtual_adapter::VirtualNetworkInterface::new().await {
                    Ok(vni) => {
                        vni.run().await;
                    }
                    Err(e) => {
                        eprintln!("Failed to create virtual network interface: {}", e);
                    }
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
