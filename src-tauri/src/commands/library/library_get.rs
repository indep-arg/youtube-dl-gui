use crate::state::library::Library;
use crate::SharedLibrary;
use tauri::State;

#[tauri::command]
pub fn library_get(library: State<'_, SharedLibrary>) -> Library {
  (*library.load()).clone()
}
