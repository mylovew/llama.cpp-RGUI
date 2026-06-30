/// 在默认浏览器中打开指定 URL
#[tauri::command]
pub async fn open_chat_window(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("无法打开浏览器: {}", e))
}
