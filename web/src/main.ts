import "./style.css";
import { bindController } from "./controller";
import { bindVisibilityToggle, lockGestures } from "./ui";

lockGestures();

const pad = document.getElementById("pad")!;
bindVisibilityToggle(document.getElementById("toggle")!, pad);

// A conexão com o PC entra na próxima funcionalidade; por ora só registra.
bindController(pad, (msg) => console.debug("vpad", [...msg]));
