use tauri::menu::MenuBuilder;
use tauri::AppHandle;

/// Windows and Linux get an empty menu bar: everything is reachable from the window itself.
pub fn setup_menu(handle: &AppHandle) {
  #[cfg(any(target_os = "windows", target_os = "linux"))]
  {
    let menu = MenuBuilder::new(handle)
      .items(&[])
      .build()
      .expect("Could not attach submenus to menu.");

    handle.set_menu(menu).expect("Could not attach menu.");
  }
  #[cfg(not(any(target_os = "windows", target_os = "linux")))]
  let _ = handle;
}
