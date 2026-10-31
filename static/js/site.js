import { initNavigation } from "./ui/navigation.js";
import { initCopyButtons } from "./ui/copy-code.js";
import { initContents } from "./ui/contents.js";

// ES modules are deferred automatically. Content and navigation work without JS.
initNavigation();
initCopyButtons();
initContents();
