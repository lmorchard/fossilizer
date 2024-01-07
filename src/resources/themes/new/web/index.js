import "./lib/theme-selector.js";
import "./lib/lazy-load-observer.js";
import "./lib/archive-nav/index.js";
import "./lib/archive-main.js";
import "./lib/archive-activity-list.js";
import "./lib/formatted-time.js";

async function handleClick(ev) {
  const { classList } = ev.target;

  if (classList.contains("media-attachment")) {
    const { fullsrc } = ev.target.dataset;
    const description = ev.target.getAttribute("title");

    const displayEl = document.querySelector("#media-attachment-display");
    displayEl.src = fullsrc;
    displayEl.setAttribute("alt", description);
    displayEl.setAttribute("title", description);

    const descriptionEl = document.querySelector(
      "#media-attachment-description"
    );
    descriptionEl.innerHTML = ev.target.getAttribute("title");
  }

}
