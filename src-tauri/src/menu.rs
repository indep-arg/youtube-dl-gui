use tauri::menu::MenuBuilder;
use tauri::AppHandle;

pub fn setup_menu(handle: &AppHandle) {
  #[cfg(target_os = "windows")]
  {
    setup_windows(handle);
  }
  #[cfg(target_os = "linux")]
  {
    setup_linux(handle);
  }
}

#[allow(dead_code)]
fn setup_windows(handle: &AppHandle) {
  let menu = MenuBuilder::new(handle)
    .items(&[])
    .build()
    .expect("Could not attach submenus to menu.");

  handle.set_menu(menu).expect("Could not attach menu.");
}

#[allow(dead_code)]
fn setup_linux(handle: &AppHandle) {
  let menu = MenuBuilder::new(handle)
    .items(&[])
    .build()
    .expect("Could not attach submenus to menu.");

  handle.set_menu(menu).expect("Could not attach menu.");
}
