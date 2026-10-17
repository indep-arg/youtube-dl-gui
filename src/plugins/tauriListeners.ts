import { registerMediaListeners } from '../tauri/listeners/media';
import { registerProgressListeners } from '../tauri/listeners/progress';
import { registerDestinationListeners } from '../tauri/listeners/destination';
import { registerBinaryListeners } from '../tauri/listeners/binaries';
import { registerUpdaterListeners } from '../tauri/listeners/updater';
import { registerDiagnosticsListeners } from '../tauri/listeners/diagnostics.ts';
import { registerShortcutListeners } from '../tauri/listeners/shortcuts.ts';

export default {
  install() {
    registerMediaListeners();
    registerProgressListeners();
    registerDestinationListeners();
    registerBinaryListeners();
    registerUpdaterListeners();
    registerDiagnosticsListeners();
    registerShortcutListeners();
  },
};
