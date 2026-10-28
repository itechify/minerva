import "./solo-exp-help.css";

/** The Map Summary arithmetic in minerva-core/src/osms.rs, shared wherever the app
 *  compares a measured rate with the map's solo ceiling. */
export function soloExpHelp() {
  return helpButton("How solo EXP/h is computed", [
    "Solo EXP/h estimates the map's hourly EXP ceiling from mob EXP and spawn points.",
    "For each mob: base EXP × spawn points × spawns per hour. Normal spawns use 0.75 × 3,600 ÷ 7.56 (about 357 per point per hour). Timed spawns use 3,600 ÷ their timer in seconds; ranged timers use OSMS's listed average. Round each mob's EXP/h, then add them.",
    "This assumes mobs are killed as they respawn, without EXP buffs or events. The map's Mob Rate isn't applied.",
  ]);
}

/** A small outlined ? button that shows `paragraphs` in a tooltip, named `label` for
 *  assistive tech: where a figure comes from. */
export function helpButton(label: string, paragraphs: string[]) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "solo-exp-help";
  button.setAttribute("aria-label", label);
  button.innerHTML = '<svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6"/><path d="M6.2 6a1.8 1.8 0 0 1 3.6 0c0 1.3-1.8 1.3-1.8 2.7M8 11.2v.1"/></svg>';

  const tip = document.createElement("div");
  tip.className = "solo-exp-tip";
  tip.id = `solo-exp-tip-${document.querySelectorAll(".solo-exp-tip").length}`;
  tip.setAttribute("role", "tooltip");
  tip.hidden = true;
  for (const text of paragraphs) {
    const p = document.createElement("p");
    p.textContent = text;
    tip.append(p);
  }
  // Outside the panel and its scrolling table, so the help isn't clipped by either.
  document.body.append(tip);
  button.setAttribute("aria-describedby", tip.id);
  let leaving: ReturnType<typeof setTimeout> | undefined;
  const stay = () => clearTimeout(leaving);
  const hide = () => {
    stay();
    tip.hidden = true;
  };
  const show = () => {
    stay();
    tip.hidden = false;
    const at = button.getBoundingClientRect();
    const width = tip.offsetWidth;
    const height = tip.offsetHeight;
    const left = Math.max(6, Math.min(at.right - width, innerWidth - width - 6));
    const below = at.bottom + 6;
    const top = below + height <= innerHeight - 6 ? below : Math.max(6, at.top - height - 6);
    tip.style.left = `${left}px`;
    tip.style.top = `${top}px`;
  };
  const leave = () => {
    if (document.activeElement === button) return;
    leaving = setTimeout(hide, 120);
  };
  button.addEventListener("pointerenter", show);
  button.addEventListener("pointerleave", leave);
  button.addEventListener("focus", show);
  button.addEventListener("blur", hide);
  button.addEventListener("click", show);
  tip.addEventListener("pointerenter", stay);
  tip.addEventListener("pointerleave", leave);
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") hide();
  });
  document.addEventListener("pointerdown", (event) => {
    if (event.target instanceof Node && !button.contains(event.target) && !tip.contains(event.target)) hide();
  });
  window.addEventListener("resize", hide);
  return button;
}
