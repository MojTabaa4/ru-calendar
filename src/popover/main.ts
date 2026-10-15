import "vazirmatn/Vazirmatn-Variable-font-face.css";
import "../styles/tokens.css";
import "../styles/base.css";

import { mount } from "svelte";
import Popover from "./Popover.svelte";

mount(Popover, { target: document.getElementById("app")! });
