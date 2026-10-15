import "vazirmatn/Vazirmatn-Variable-font-face.css";
import "../styles/tokens.css";
import "../styles/base.css";

import { mount } from "svelte";
import Converter from "./Converter.svelte";

mount(Converter, { target: document.getElementById("app")! });
