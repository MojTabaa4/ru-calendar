import "vazirmatn/Vazirmatn-Variable-font-face.css";
import "../styles/tokens.css";
import "../styles/base.css";

import { mount } from "svelte";
import Settings from "./Settings.svelte";

mount(Settings, { target: document.getElementById("app")! });
