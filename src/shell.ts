/** What the windows do alike with the shell: tell it their height, drag by their header,
 *  press the core's controls and follow the view it sends. (Apart from dom.ts, which the
 *  site shares and so must not bring Tauri with it.) */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * The shell sizes a window to its content, through `command`. The content is no taller than
 * the window, so it is measured without that limit (its `measuring` class), or the window
 * could never grow; a height is reported only once it changes. Lifting the limit scrolls
 * the content's long parts back to the top, so with `keepScrolled` each `.scroll` in it is
 * put back where it was.
 */
export function heightReporter(el: HTMLElement, command: string, keepScrolled = false): () => void {
  let reported = 0;
  return () => {
    const scrolls = keepScrolled ? [...el.querySelectorAll<HTMLElement>(".scroll")] : [];
    const scrolledTo = scrolls.map((s) => s.scrollTop);
    el.classList.add("measuring");
    const height = Math.ceil(el.getBoundingClientRect().height);
    el.classList.remove("measuring");
    scrolls.forEach((s, i) => (s.scrollTop = scrolledTo[i]));
    if (height > 0 && height !== reported) {
      reported = height;
      invoke(command, { height });
    }
  };
}

/** The window is dragged by its `.drag-handle`s, never by what in them is `notOn`, and
 *  only while `when` says so. */
export function dragByHandles(notOn = "button", when: (target: Element) => boolean = () => true) {
  document.addEventListener("mousedown", (e) => {
    const target = e.target instanceof Element ? e.target : null;
    const onHandle = !!target?.closest(".drag-handle") && !target?.closest(notOn);
    if (e.button === 0 && onHandle && when(target!)) getCurrentWindow().startDragging();
  });
}

/** A click on a `button[data-control]` in `root` presses that control, through the core. */
export function pressControls(root: Node) {
  root.addEventListener("click", (e) => {
    const button = e.target instanceof Element ? e.target.closest<HTMLButtonElement>("button[data-control]") : null;
    if (button) invoke("panel_press", { control: button.dataset.control });
  });
}

/** Draws each view the shell sends as `event`, and the one it holds already (`command`): it
 *  may have been sent before the page was listening. */
export function followView<T>(event: string, command: string, render: (view: T) => void) {
  listen<T>(event, (e) => render(e.payload));
  invoke<T | null>(command).then((view) => {
    if (view) render(view);
  });
}
